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

use rpc::admin_cli::{CarbideCliError, CarbideCliResult};
use rpc::protos::mlx_device as mlx_device_pb;

pub mod args {
    use carbide_uuid::machine::MachineId;
    use clap::Parser;

    use super::*;

    // config/args.rs
    // Command-line argument definitions for config commands.

    // ConfigCommand are the config subcommands.
    #[derive(Parser, Debug)]
    pub enum ConfigCommand {
        #[clap(about = "Query device configuration values")]
        Query(ConfigQueryCommand),

        #[clap(about = "Set device configuration values")]
        Set(ConfigSetCommand),

        #[clap(about = "Synchronize configuration values to a device")]
        Sync(ConfigSyncCommand),

        #[clap(about = "Compare device configuration against expected values")]
        Compare(ConfigCompareCommand),
    }

    // ConfigQueryCommand queries device configuration values.
    #[derive(Parser, Debug)]
    pub struct ConfigQueryCommand {
        #[arg(help = "Carbide Machine ID")]
        pub machine_id: MachineId,

        #[arg(help = "Device ID is the PCI or mst path on the target machine")]
        pub device_id: String,

        // registry_name is the registry to use.
        #[arg(help = "Backing variable registry to query against")]
        pub registry_name: String,
        // variables are optional specific variables to query.
        #[arg(help = "Variables to query, all if unset.", value_delimiter = ',')]
        pub variables: Vec<String>,
    }

    // ConfigSetCommand sets device configuration values.
    #[derive(Parser, Debug)]
    pub struct ConfigSetCommand {
        #[arg(help = "Carbide Machine ID")]
        pub machine_id: MachineId,

        #[arg(help = "Device ID is the PCI or mst path on the target machine")]
        pub device_id: String,

        // registry_name is the registry to use.
        pub registry_name: String,
        // assignments are variable=value assignments.
        #[arg(value_delimiter = ',')]
        pub assignments: Vec<String>,
    }

    // ConfigSyncCommand synchronizes configuration values to a device.
    #[derive(Parser, Debug)]
    pub struct ConfigSyncCommand {
        #[arg(help = "Carbide Machine ID")]
        pub machine_id: MachineId,

        #[arg(help = "Device ID is the PCI or mst path on the target machine")]
        pub device_id: String,

        // registry_name is the registry to use.
        pub registry_name: String,
        // assignments are variable=value assignments.
        #[arg(value_delimiter = ',')]
        pub assignments: Vec<String>,
    }

    // ConfigCompareCommand compares device configuration against expected values.
    #[derive(Parser, Debug)]
    pub struct ConfigCompareCommand {
        #[arg(help = "Carbide Machine ID")]
        pub machine_id: MachineId,

        #[arg(help = "Device ID is the PCI or mst path on the target machine")]
        pub device_id: String,

        // registry_name is the registry to use.
        pub registry_name: String,
        // assignments are variable=value assignments.
        #[arg(value_delimiter = ',')]
        pub assignments: Vec<String>,
    }

    impl From<ConfigQueryCommand> for mlx_device_pb::MlxAdminConfigQueryRequest {
        fn from(cmd: ConfigQueryCommand) -> Self {
            Self {
                machine_id: cmd.machine_id.into(),
                device_id: cmd.device_id,
                registry_name: cmd.registry_name,
                variables: cmd.variables,
            }
        }
    }

    impl TryFrom<ConfigSetCommand> for mlx_device_pb::MlxAdminConfigSetRequest {
        type Error = CarbideCliError;

        fn try_from(cmd: ConfigSetCommand) -> Result<Self, Self::Error> {
            let parsed_assignments = parse_assignments(&cmd.assignments)?;
            Ok(Self {
                machine_id: cmd.machine_id.into(),
                device_id: cmd.device_id,
                registry_name: cmd.registry_name,
                assignments: parsed_assignments,
            })
        }
    }

    impl TryFrom<ConfigSyncCommand> for mlx_device_pb::MlxAdminConfigSyncRequest {
        type Error = CarbideCliError;

        fn try_from(cmd: ConfigSyncCommand) -> Result<Self, Self::Error> {
            let parsed_assignments = parse_assignments(&cmd.assignments)?;
            Ok(Self {
                machine_id: cmd.machine_id.into(),
                device_id: cmd.device_id,
                registry_name: cmd.registry_name,
                assignments: parsed_assignments,
            })
        }
    }

    impl TryFrom<ConfigCompareCommand> for mlx_device_pb::MlxAdminConfigCompareRequest {
        type Error = CarbideCliError;

        fn try_from(cmd: ConfigCompareCommand) -> Result<Self, Self::Error> {
            let parsed_assignments = parse_assignments(&cmd.assignments)?;
            Ok(Self {
                machine_id: cmd.machine_id.into(),
                device_id: cmd.device_id,
                registry_name: cmd.registry_name,
                assignments: parsed_assignments,
            })
        }
    }

    // parse_assignments is a helper to parse "var=value" assignments.
    fn parse_assignments(
        assignments: &[String],
    ) -> CarbideCliResult<Vec<mlx_device_pb::VariableAssignment>> {
        let mut result = Vec::new();

        for assignment in assignments {
            let parts: Vec<&str> = assignment.splitn(2, '=').collect();
            if parts.len() != 2 {
                return Err(CarbideCliError::GenericError(format!(
                    "invalid assignment format: {assignment} (expected: variable=value)"
                )));
            }

            result.push(mlx_device_pb::VariableAssignment {
                variable_name: parts[0].to_string(),
                value: parts[1].to_string(),
            });
        }

        Ok(result)
    }
}

pub mod cmds {
    use ::rpc::admin_cli::OutputFormat;
    use libmlx::runner::result_types::{ComparisonResult, QueryResult, SyncResult};
    use prettytable::{Cell, Row, Table};

    use super::args::{
        ConfigCommand, ConfigCompareCommand, ConfigQueryCommand, ConfigSetCommand,
        ConfigSyncCommand,
    };
    use super::*;
    use crate::mlx::{
        CliContext, print_comparison_result_csv, print_comparison_result_table,
        print_sync_result_csv, print_sync_result_table, wrap_text,
    };

    // dispatch routes config subcommands to its handlers.
    pub async fn dispatch(
        command: ConfigCommand,
        ctxt: &mut CliContext<'_, '_>,
    ) -> CarbideCliResult<()> {
        match command {
            ConfigCommand::Query(cmd) => handle_query(cmd, ctxt).await,
            ConfigCommand::Set(cmd) => handle_set(cmd, ctxt).await,
            ConfigCommand::Sync(cmd) => handle_sync(cmd, ctxt).await,
            ConfigCommand::Compare(cmd) => handle_compare(cmd, ctxt).await,
        }
    }
    async fn handle_query(
        cmd: ConfigQueryCommand,
        ctxt: &mut CliContext<'_, '_>,
    ) -> CarbideCliResult<()> {
        let request: mlx_device_pb::MlxAdminConfigQueryRequest = cmd.into();
        let response = ctxt.grpc_conn.0.mlx_admin_config_query(request).await?;

        let query_result_pb = response
            .query_result
            .ok_or_else(|| CarbideCliError::GenericError("no query result returned".to_string()))?;

        let query_result: QueryResult = query_result_pb.try_into()?;

        match ctxt.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&query_result)?);
            }
            OutputFormat::Yaml => {
                println!("{}", serde_yaml::to_string(&query_result)?);
            }
            OutputFormat::AsciiTable => {
                print_query_result_table(&query_result);
            }
            OutputFormat::Csv => {
                println!("CSV not supported yet")
            }
        }

        Ok(())
    }

    async fn handle_set(
        cmd: ConfigSetCommand,
        ctxt: &mut CliContext<'_, '_>,
    ) -> CarbideCliResult<()> {
        let request: mlx_device_pb::MlxAdminConfigSetRequest = cmd.try_into()?;
        let response = ctxt.grpc_conn.0.mlx_admin_config_set(request).await?;

        println!(
            "Successfully applied {} variable assignments.",
            response.total_applied
        );
        Ok(())
    }

    async fn handle_sync(
        cmd: ConfigSyncCommand,
        ctxt: &mut CliContext<'_, '_>,
    ) -> CarbideCliResult<()> {
        let request: mlx_device_pb::MlxAdminConfigSyncRequest = cmd.try_into()?;
        let response = ctxt.grpc_conn.0.mlx_admin_config_sync(request).await?;

        let sync_result_pb = response
            .sync_result
            .ok_or_else(|| CarbideCliError::GenericError("no sync result returned".to_string()))?;

        let sync_result: SyncResult = sync_result_pb.try_into()?;

        match ctxt.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&sync_result)?);
            }
            OutputFormat::Yaml => {
                println!("{}", serde_yaml::to_string(&sync_result)?);
            }
            OutputFormat::AsciiTable => {
                print_sync_result_table(&sync_result);
            }
            OutputFormat::Csv => {
                print_sync_result_csv(&sync_result);
            }
        }

        Ok(())
    }

    async fn handle_compare(
        cmd: ConfigCompareCommand,
        ctxt: &mut CliContext<'_, '_>,
    ) -> CarbideCliResult<()> {
        let request: mlx_device_pb::MlxAdminConfigCompareRequest = cmd.try_into()?;
        let response = ctxt.grpc_conn.0.mlx_admin_config_compare(request).await?;

        let comparison_result_pb = response.comparison_result.ok_or_else(|| {
            CarbideCliError::GenericError("no comparison result returned".to_string())
        })?;

        let comparison_result: ComparisonResult = comparison_result_pb.try_into()?;

        // Output the comparison result in the requested format.
        match ctxt.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&comparison_result)?);
            }
            OutputFormat::Yaml => {
                println!("{}", serde_yaml::to_string(&comparison_result)?);
            }
            OutputFormat::AsciiTable => {
                print_comparison_result_table(&comparison_result);
            }
            OutputFormat::Csv => {
                print_comparison_result_csv(&comparison_result);
            }
        }

        Ok(())
    }

    // print_query_result_table displays a QueryResult in ASCII table format.
    fn print_query_result_table(result: &QueryResult) {
        let mut table = Table::new();

        // Add header row.
        table.add_row(Row::new(vec![
            Cell::new("Variable"),
            Cell::new("Current"),
            Cell::new("Next"),
            Cell::new("Default"),
            Cell::new("Modified"),
            Cell::new("Read-Only"),
        ]));

        // Add variable rows.
        for var in &result.variables {
            let modified_str = if var.modified { "Yes" } else { "No" };
            let read_only_str = if var.read_only { "Yes" } else { "No" };

            let wrapped_current = wrap_text(&var.current_value.to_string(), 60);
            let wrapped_next = wrap_text(&var.next_value.to_string(), 60);
            let wrapped_default = wrap_text(&var.default_value.to_string(), 60);

            table.add_row(Row::new(vec![
                Cell::new(&var.variable.name),
                Cell::new(&wrapped_current),
                Cell::new(&wrapped_next),
                Cell::new(&wrapped_default),
                Cell::new(modified_str),
                Cell::new(read_only_str),
            ]));
        }

        table.printstd();
    }
}
