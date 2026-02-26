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

use rpc::protos::mlx_device as mlx_device_pb;

pub mod args {
    use carbide_uuid::machine::MachineId;
    use clap::Parser;

    use super::*;

    // registry/args.rs
    // Command-line argument definitions for registry commands.

    // RegistryCommand are the registry subcommands.
    #[derive(Parser, Debug)]
    pub enum RegistryCommand {
        #[clap(about = "List all available registries")]
        List(RegistryListCommand),

        #[clap(about = "Show details of a specific registry")]
        Show(RegistryShowCommand),
    }

    // RegistryListCommand lists all available registries.
    #[derive(Parser, Debug)]
    pub struct RegistryListCommand {
        #[arg(help = "Carbide Machine ID")]
        pub machine_id: MachineId,
    }

    // RegistryShowCommand shows details of a specific registry.
    #[derive(Parser, Debug)]
    pub struct RegistryShowCommand {
        #[arg(help = "Carbide Machine ID")]
        pub machine_id: MachineId,

        #[arg(help = "Registry name to show")]
        pub registry_name: String,
    }

    impl From<RegistryListCommand> for mlx_device_pb::MlxAdminRegistryListRequest {
        fn from(cmd: RegistryListCommand) -> Self {
            Self {
                machine_id: cmd.machine_id.into(),
            }
        }
    }

    impl From<RegistryShowCommand> for mlx_device_pb::MlxAdminRegistryShowRequest {
        fn from(cmd: RegistryShowCommand) -> Self {
            Self {
                machine_id: cmd.machine_id.into(),
                registry_name: cmd.registry_name,
            }
        }
    }
}

pub mod cmds {
    use libmlx::variables::registry::MlxVariableRegistry;
    use prettytable::{Cell, Row, Table};
    use rpc::admin_cli::{CarbideCliError, CarbideCliResult, OutputFormat};

    use super::args::{RegistryCommand, RegistryListCommand, RegistryShowCommand};
    use super::*;
    use crate::mlx::{CliContext, wrap_text};

    // registry/cmds.rs
    // Command handlers for registry operations.

    // dispatch routes registry subcommands to their handlers.
    pub async fn dispatch(
        command: RegistryCommand,
        ctxt: &mut CliContext<'_, '_>,
    ) -> CarbideCliResult<()> {
        match command {
            RegistryCommand::List(cmd) => handle_list(cmd, ctxt).await,
            RegistryCommand::Show(cmd) => handle_show(cmd, ctxt).await,
        }
    }

    // handle_list lists all registries configured in the remote scout agent.
    async fn handle_list(
        cmd: RegistryListCommand,
        ctxt: &mut CliContext<'_, '_>,
    ) -> CarbideCliResult<()> {
        let request: mlx_device_pb::MlxAdminRegistryListRequest = cmd.into();
        let response = ctxt.grpc_conn.0.mlx_admin_registry_list(request).await?;

        let registry_listing = response.registry_listing.ok_or_else(|| {
            CarbideCliError::GenericError("no registry listing returned".to_string())
        })?;

        let mut registry_names = registry_listing.registry_names;
        registry_names.sort();

        match ctxt.format {
            OutputFormat::AsciiTable => {
                let mut table = Table::new();
                table.add_row(Row::new(vec![Cell::new("Registry Name")]));

                for name in &registry_names {
                    table.add_row(Row::new(vec![Cell::new(name)]));
                }

                table.printstd();
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&registry_names)?;
                println!("{json}");
            }
            OutputFormat::Yaml => {
                println!("registries:");
                for name in registry_names {
                    println!("  - {name}");
                }
            }
            OutputFormat::Csv => {
                for name in registry_names {
                    println!("{name}");
                }
            }
        }

        Ok(())
    }

    // handle_show shows a profile configured in carbide-api.
    async fn handle_show(
        cmd: RegistryShowCommand,
        ctxt: &mut CliContext<'_, '_>,
    ) -> CarbideCliResult<()> {
        let request: mlx_device_pb::MlxAdminRegistryShowRequest = cmd.into();
        let response = ctxt.grpc_conn.0.mlx_admin_registry_show(request).await?;

        let variable_registry_pb = response.variable_registry.ok_or_else(|| {
            CarbideCliError::GenericError("no variable_registry returned".to_string())
        })?;

        let variable_registry: MlxVariableRegistry = variable_registry_pb.try_into()?;

        match ctxt.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&variable_registry)?);
            }
            OutputFormat::Yaml => {
                println!("{}", serde_yaml::to_string(&variable_registry)?);
            }
            OutputFormat::AsciiTable => {
                print_registry_table(&variable_registry);
            }
            OutputFormat::Csv => {
                println!("CSV not yet supported");
            }
        }

        Ok(())
    }

    // print_registry_table displays a registry in ASCII table format.
    fn print_registry_table(registry: &MlxVariableRegistry) {
        let mut table = Table::new();

        // Header: Registry name.
        println!("Registry: {}", registry.name);
        println!();

        // Add variable table header.
        table.add_row(Row::new(vec![
            Cell::new("Variable"),
            Cell::new("Type"),
            Cell::new("RW"),
            Cell::new("Description"),
        ]));

        // Add variable rows.
        for variable in &registry.variables {
            let rw = if variable.read_only { "RO" } else { "RW" };

            let wrapped_description = wrap_text(&variable.description, 60);

            table.add_row(Row::new(vec![
                Cell::new(&variable.name),
                Cell::new(&variable.spec.to_string()),
                Cell::new(rw),
                Cell::new(&wrapped_description),
            ]));
        }

        table.printstd();
    }
}
