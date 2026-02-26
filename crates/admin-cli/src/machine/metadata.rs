/*
 * SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use ::rpc::admin_cli::CarbideCliResult;
pub use args::Args;

use crate::cfg::run::Run;
use crate::cfg::runtime::RuntimeContext;

pub mod args {
    use carbide_uuid::machine::MachineId;
    use clap::Parser;

    #[derive(Parser, Debug, Clone)]
    pub enum Args {
        #[clap(about = "Set the Name or Description of the Machine")]
        Set(MachineMetadataCommandSet),
        #[clap(about = "Show the Metadata of the Machine")]
        Show(MachineMetadataCommandShow),
        #[clap(about = "Adds a label to the Metadata of a Machine")]
        AddLabel(MachineMetadataCommandAddLabel),
        #[clap(about = "Removes labels from the Metadata of a Machine")]
        RemoveLabels(MachineMetadataCommandRemoveLabels),
        #[clap(about = "Copy Machine Metadata from Expected-Machine to Machine")]
        FromExpectedMachine(MachineMetadataCommandFromExpectedMachine),
    }

    #[derive(Parser, Debug, Clone)]
    pub struct MachineMetadataCommandShow {
        #[clap(help = "The machine which should get updated metadata")]
        pub machine: MachineId,
    }

    #[derive(Parser, Debug, Clone)]
    pub struct MachineMetadataCommandSet {
        #[clap(help = "The machine which should get updated metadata")]
        pub machine: MachineId,
        #[clap(long, help = "The updated name of the Machine")]
        pub name: Option<String>,
        #[clap(long, help = "The updated description of the Machine")]
        pub description: Option<String>,
    }

    #[derive(Parser, Debug, Clone)]
    pub struct MachineMetadataCommandAddLabel {
        #[clap(help = "The machine which should get updated metadata")]
        pub machine: MachineId,
        #[clap(long, help = "The key to add")]
        pub key: String,
        #[clap(long, help = "The optional value to add")]
        pub value: Option<String>,
    }

    #[derive(Parser, Debug, Clone)]
    pub struct MachineMetadataCommandRemoveLabels {
        #[clap(help = "The machine which should get updated metadata")]
        pub machine: MachineId,
        #[clap(long, help = "The keys to remove")]
        pub keys: Vec<String>,
    }

    #[derive(Parser, Debug, Clone)]
    pub struct MachineMetadataCommandFromExpectedMachine {
        #[clap(help = "The machine which should get updated metadata")]
        pub machine: MachineId,
        /// Whether to fully replace the Metadata that is currently stored on the Machine.
        /// - If not set, existing Metadata on the Machine will not be touched by executing
        ///   the command:
        ///   - The existing Name will not be changed if the Name is not equivalent
        ///     to the Machine ID or Empty.
        ///   - The existing Description will not be changed if it is not empty.
        ///   - Existing Labels and their values will not be changed. Only labels which
        ///     do not exist on the Machine will be added.
        /// - If set, the Machines Metadata will be set to the same values as
        ///   they would if the Machine would get freshly ingested.
        ///   Metadata that is currently set on the Machine will be overridden.
        #[clap(long, verbatim_doc_comment)]
        pub replace_all: bool,
    }
}

pub mod cmd {
    use std::collections::HashSet;
    use std::pin::Pin;

    use ::rpc::admin_cli::{CarbideCliError, OutputFormat};
    use mac_address::MacAddress;
    use prettytable::{Row, Table};
    use rpc::Machine;

    use super::args::{
        Args, MachineMetadataCommandAddLabel, MachineMetadataCommandFromExpectedMachine,
        MachineMetadataCommandRemoveLabels, MachineMetadataCommandSet, MachineMetadataCommandShow,
    };
    use super::*;
    use crate::rpc::ApiClient;
    use crate::{async_write, async_writeln};

    pub async fn metadata(
        api_client: &ApiClient,
        cmd: Args,
        output_file: &mut Pin<Box<dyn tokio::io::AsyncWrite>>,
        format: OutputFormat,
        extended: bool,
    ) -> CarbideCliResult<()> {
        match cmd {
            Args::Show(cmd) => metadata_show(api_client, cmd, output_file, format, extended).await,
            Args::Set(cmd) => metadata_set(api_client, cmd).await,
            Args::AddLabel(cmd) => metadata_add_label(api_client, cmd).await,
            Args::RemoveLabels(cmd) => metadata_remove_labels(api_client, cmd).await,
            Args::FromExpectedMachine(cmd) => metadata_from_expected_machine(api_client, cmd).await,
        }
    }

    pub async fn handle_metadata_show(
        output_file: &mut Pin<Box<dyn tokio::io::AsyncWrite>>,
        output_format: &OutputFormat,
        _extended: bool,
        machine: Machine,
    ) -> CarbideCliResult<()> {
        let metadata = machine.metadata.ok_or(CarbideCliError::Empty)?;

        match output_format {
            OutputFormat::AsciiTable => {
                async_writeln!(output_file, "Name        : {}", metadata.name)?;
                async_writeln!(output_file, "Description : {}", metadata.description)?;
                let mut table = Table::new();
                table.set_titles(Row::from(vec!["Key", "Value"]));
                for l in &metadata.labels {
                    table.add_row(Row::from(vec![&l.key, l.value.as_deref().unwrap_or("")]));
                }
                async_write!(output_file, "{}", table)?;
            }
            OutputFormat::Csv => {
                return Err(CarbideCliError::NotImplemented(
                    "CSV formatted output".to_string(),
                ));
            }
            OutputFormat::Json => {
                async_writeln!(output_file, "{}", serde_json::to_string_pretty(&metadata)?)?
            }
            OutputFormat::Yaml => {
                return Err(CarbideCliError::NotImplemented(
                    "YAML formatted output".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub async fn metadata_show(
        api_client: &ApiClient,
        cmd: MachineMetadataCommandShow,
        output_file: &mut Pin<Box<dyn tokio::io::AsyncWrite>>,
        format: OutputFormat,
        extended: bool,
    ) -> CarbideCliResult<()> {
        let mut machines = api_client
            .get_machines_by_ids(&[cmd.machine])
            .await?
            .machines;
        let Some(machine) = machines.pop() else {
            return Err(CarbideCliError::GenericError(format!(
                "Machine with ID {} was not found",
                cmd.machine
            )));
        };
        handle_metadata_show(output_file, &format, extended, machine).await
    }

    pub async fn metadata_set(
        api_client: &ApiClient,
        cmd: MachineMetadataCommandSet,
    ) -> CarbideCliResult<()> {
        let mut machines = api_client
            .get_machines_by_ids(&[cmd.machine])
            .await?
            .machines;
        if machines.len() != 1 {
            return Err(CarbideCliError::GenericError(format!(
                "Machine with ID {} was not found",
                cmd.machine
            )));
        }
        let machine = machines.remove(0);

        let mut metadata = machine.metadata.ok_or_else(|| {
            CarbideCliError::GenericError(
                "Machine does not carry Metadata that can be patched".into(),
            )
        })?;
        if let Some(name) = cmd.name {
            metadata.name = name;
        }
        if let Some(description) = cmd.description {
            metadata.description = description;
        }

        api_client
            .update_machine_metadata(machine.id.unwrap(), metadata, machine.version)
            .await?;
        Ok(())
    }

    pub async fn metadata_add_label(
        api_client: &ApiClient,
        cmd: MachineMetadataCommandAddLabel,
    ) -> CarbideCliResult<()> {
        let mut machines = api_client
            .get_machines_by_ids(&[cmd.machine])
            .await?
            .machines;
        if machines.len() != 1 {
            return Err(CarbideCliError::GenericError(format!(
                "Machine with ID {} was not found",
                cmd.machine
            )));
        }
        let machine = machines.remove(0);

        let mut metadata = machine.metadata.ok_or_else(|| {
            CarbideCliError::GenericError(
                "Machine does not carry Metadata that can be patched".into(),
            )
        })?;
        metadata.labels.retain_mut(|l| l.key != cmd.key);
        metadata.labels.push(::rpc::forge::Label {
            key: cmd.key,
            value: cmd.value,
        });

        api_client
            .update_machine_metadata(machine.id.unwrap(), metadata, machine.version)
            .await?;
        Ok(())
    }

    pub async fn metadata_remove_labels(
        api_client: &ApiClient,
        cmd: MachineMetadataCommandRemoveLabels,
    ) -> CarbideCliResult<()> {
        let mut machines = api_client
            .get_machines_by_ids(&[cmd.machine])
            .await?
            .machines;
        if machines.len() != 1 {
            return Err(CarbideCliError::GenericError(format!(
                "Machine with ID {} was not found",
                cmd.machine
            )));
        }
        let machine = machines.remove(0);

        let mut metadata = machine.metadata.ok_or_else(|| {
            CarbideCliError::GenericError(
                "Machine does not carry Metadata that can be patched".into(),
            )
        })?;

        // Retain everything that isn't specified as removed
        let removed_labels: HashSet<String> = cmd.keys.into_iter().collect();
        metadata.labels.retain(|l| !removed_labels.contains(&l.key));

        api_client
            .update_machine_metadata(machine.id.unwrap(), metadata, machine.version)
            .await?;
        Ok(())
    }

    pub async fn metadata_from_expected_machine(
        api_client: &ApiClient,
        cmd: MachineMetadataCommandFromExpectedMachine,
    ) -> CarbideCliResult<()> {
        let mut machines = api_client
            .get_machines_by_ids(&[cmd.machine])
            .await?
            .machines;
        if machines.len() != 1 {
            return Err(CarbideCliError::GenericError(format!(
                "Machine with ID {} was not found",
                cmd.machine
            )));
        }
        let machine = machines.remove(0);
        let bmc_mac: MacAddress = machine
            .bmc_info
            .and_then(|bmc_info| bmc_info.mac)
            .map(|mac| mac.parse())
            .transpose()
            .map_or_else(
                |e| {
                    Err(CarbideCliError::GenericError(format!(
                        "Invalid BMC MAC address found for Machine with ID {}: {}",
                        cmd.machine, e
                    )))
                },
                Ok,
            )?
            .ok_or_else(|| {
                CarbideCliError::GenericError(format!(
                    "No BMC MAC address found for Machine with ID {}",
                    cmd.machine
                ))
            })?;

        let mut metadata = machine.metadata.ok_or_else(|| {
            CarbideCliError::GenericError(
                "Machine does not carry Metadata that can be patched".into(),
            )
        })?;

        let expected_machines = api_client
            .0
            .get_all_expected_machines()
            .await?
            .expected_machines;
        let expected_machine = expected_machines
            .into_iter()
            .find(|em| {
                em.bmc_mac_address
                    .parse::<MacAddress>()
                    .is_ok_and(|m| m == bmc_mac)
            })
            .ok_or_else(|| {
                CarbideCliError::GenericError(format!(
                    "No expected Machine found for Machine with ID {} and BMC Mac address {}",
                    cmd.machine, bmc_mac
                ))
            })?;

        let expected_machine_metadata = expected_machine.metadata.ok_or_else(|| {
            CarbideCliError::GenericError(format!(
                "No expected Machine Metadata found for Machine with ID {} and BMC Mac address {}",
                cmd.machine, bmc_mac
            ))
        })?;

        if cmd.replace_all {
            // Configure the Machines metadata in the same way as if the Machine was freshly ingested
            metadata.name = if expected_machine_metadata.name.is_empty() {
                machine.id.map(|id| id.to_string()).unwrap_or_default()
            } else {
                expected_machine_metadata.name
            };
            metadata.description = expected_machine_metadata.description;
            metadata.labels = expected_machine_metadata.labels;
        } else {
            // Add new data from expected-machines, but current values that might have been the
            // result of previous changed to the Machine.
            // This operation is lossless for existing Metadata.
            if !expected_machine_metadata.name.is_empty()
                && (metadata.name.is_empty() || metadata.name == cmd.machine.to_string())
            {
                metadata.name = expected_machine_metadata.name;
            };
            if !expected_machine_metadata.description.is_empty() && metadata.description.is_empty()
            {
                metadata.description = expected_machine_metadata.description;
            };
            for label in expected_machine_metadata.labels {
                if !metadata.labels.iter().any(|l| l.key == label.key) {
                    metadata.labels.push(label);
                }
            }
        }

        api_client
            .update_machine_metadata(machine.id.unwrap(), metadata, machine.version)
            .await?;
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::metadata(
            &ctx.api_client,
            self,
            &mut ctx.output_file,
            ctx.config.format,
            ctx.config.extended,
        )
        .await?;
        Ok(())
    }
}
