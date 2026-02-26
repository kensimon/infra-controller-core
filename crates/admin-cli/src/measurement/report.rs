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

//!
//! Measured Boot CLI-backing args & commands for the `measurement report`
//! subcommand.
//!

/**
 *  Measured Boot CLI arguments for the `measurement report` subcommand.
 *
 * This provides the CLI subcommands and arguments for:
 *  - `report create`: Create a new machine measurement report.
 *  - `report delete`: Delete an existing machine measurement report.
 *  - `report promote`: Promote a machine measurement report to a bundle.
 *  - `report revoke`: Create a revoked measurement bundle from a report.
 *  - `report show all`: Show all info about all measurement reports.
 *  - `report show id`: Show all info about a specific report.
 *  - `report show machine`: Show all info about reports for a given machine.
 *  - `report list all`: List high level info about all reports.
 *  - `report list machine`: List all reports for a given machine.
 *  - `report match``
*/
pub mod args {
    use carbide_uuid::machine::MachineId;
    use carbide_uuid::measured_boot::MeasurementReportId;
    use clap::Parser;
    use measured_boot::pcr::{PcrRegisterValue, PcrSet, parse_pcr_index_input};

    use crate::cfg::measurement::parse_pcr_register_values;

    // CmdReport provides a container for the `report`
    // subcommand, which itself contains other subcommands
    // for working with reports.
    #[derive(Parser, Debug)]
    pub enum CmdReport {
        #[clap(
            about = "Create a new report with a given config.",
            visible_alias = "c"
        )]
        Create(Create),

        #[clap(about = "Delete a report by ID.", visible_alias = "d")]
        Delete(Delete),

        #[clap(
            about = "Promote a specific journal entry to an active bundle",
            visible_alias = "p"
        )]
        Promote(Promote),

        #[clap(
            about = "Mark a specific journal entry as a revoked bundle.",
            visible_alias = "r"
        )]
        Revoke(Revoke),

        #[clap(
            subcommand,
            about = "Show reports in different ways.",
            visible_alias = "s"
        )]
        Show(ShowFor),

        #[clap(
            subcommand,
            about = "List reports by various ways.",
            visible_alias = "l"
        )]
        List(List),

        #[clap(
            about = "Match reports with the provided PCR register values.",
            visible_alias = "m"
        )]
        Match(Match),
    }

    /// Create is used for creating reports, which really
    /// should be happening during machine attestation.
    #[derive(Parser, Debug)]
    pub struct Create {
        #[clap(help = "The machine ID of the machine to associate this report with.")]
        pub machine_id: MachineId,

        #[clap(
            required = true,
            use_value_delimiter = true,
            value_delimiter = ',',
            help = "Comma-separated list of {pcr_register:value,...} to associate with this report."
        )]
        #[arg(value_parser = parse_pcr_register_values)]
        pub values: Vec<PcrRegisterValue>,
    }

    /// Delete a profile by ID.
    #[derive(Parser, Debug)]
    pub struct Delete {
        #[clap(help = "The report ID.")]
        pub report_id: MeasurementReportId,
    }

    /// Promote is used to promote a report to a measurement bundle,
    /// with the ability to select which PCR registers to select from the
    /// report to use for creating the new bundle.
    #[derive(Parser, Debug)]
    pub struct Promote {
        #[clap(help = "The report ID to promote.")]
        pub report_id: MeasurementReportId,

        #[clap(
            long,
            help = "Select a specific PCR range to use for the promoted bundle."
        )]
        #[arg(value_parser = parse_pcr_index_input)]
        pub pcr_registers: Option<PcrSet>,
    }

    /// Revoke is used to mark a report as a revoked measurement bundle,
    /// with the ability to select which PCR registers to select from the
    /// report to use for creating the new (and revoked) bundle.
    #[derive(Parser, Debug)]
    pub struct Revoke {
        #[clap(help = "The report ID to revoke.")]
        pub report_id: MeasurementReportId,

        #[clap(
            long,
            help = "Select a specific PCR range to use for the revoked bundle."
        )]
        #[arg(value_parser = parse_pcr_index_input)]
        pub pcr_registers: Option<PcrSet>,
    }

    /// Show a report for an ID, reports for a machine, or all reports.
    #[derive(Parser, Debug)]
    pub enum ShowFor {
        #[clap(about = "Show a report ID.")]
        Id(ShowForId),

        #[clap(about = "Show reports for a machine.")]
        Machine(ShowForMachine),

        #[clap(about = "Show all reports.")]
        All,
    }

    /// Show a report for the given ID.
    #[derive(Parser, Debug)]
    pub struct ShowForId {
        #[clap(help = "The report ID.")]
        pub report_id: MeasurementReportId,
    }

    /// Show all reports for a machine.
    #[derive(Parser, Debug)]
    pub struct ShowForMachine {
        #[clap(help = "The profile name.")]
        pub machine_id: String,
    }

    /// List provides a few ways to list things.
    #[derive(Parser, Debug)]
    pub enum List {
        #[clap(about = "List all reports", visible_alias = "a")]
        All(ListAll),

        #[clap(
            about = "List all reports for a given machine ID.",
            visible_alias = "m"
        )]
        Machines(ListMachines),
    }

    /// ListAll will list all profiles.
    #[derive(Parser, Debug)]
    pub struct ListAll {}

    /// ListMachines will list all machines matching this report.
    #[derive(Parser, Debug)]
    pub struct ListMachines {
        #[clap(help = "The machine ID.")]
        pub machine_id: MachineId,
    }

    /// Match is used for finding reports matching the provided PCR pairs.
    #[derive(Parser, Debug)]
    pub struct Match {
        #[clap(
            required = true,
            use_value_delimiter = true,
            value_delimiter = ',',
            help = "Comma-separated list of {pcr_register:value,...} to match on."
        )]
        #[arg(value_parser = parse_pcr_register_values)]
        pub values: Vec<PcrRegisterValue>,
    }
}

/// `measurement report` subcommand dispatcher + backing functions.
pub mod cmds {
    use ::rpc::admin_cli::{CarbideCliError, CarbideCliResult, ToTable, cli_output};
    use ::rpc::protos::measured_boot::{
        CreateMeasurementReportRequest, DeleteMeasurementReportRequest,
        ListMeasurementReportRequest, MatchMeasurementReportRequest,
        PromoteMeasurementReportRequest, RevokeMeasurementReportRequest,
        ShowMeasurementReportForIdRequest, ShowMeasurementReportsForMachineRequest,
        list_measurement_report_request,
    };
    use measured_boot::bundle::MeasurementBundle;
    use measured_boot::records::MeasurementReportRecord;
    use measured_boot::report::MeasurementReport;
    use serde::Serialize;

    use crate::measurement::global;
    use crate::measurement::report::args::{
        CmdReport, Create, Delete, List, ListMachines, Match, Promote, Revoke, ShowFor, ShowForId,
        ShowForMachine,
    };
    use crate::rpc::ApiClient;

    /// dispatch matches + dispatches the correct command for
    /// the `bundle` subcommand (e.g. create, delete, set-state).
    pub async fn dispatch(
        cmd: CmdReport,
        cli: &mut global::cmds::CliData<'_, '_>,
    ) -> CarbideCliResult<()> {
        match cmd {
            CmdReport::Create(local_args) => {
                cli_output(
                    create_for_id(cli.grpc_conn, local_args).await?,
                    &cli.args.format,
                    ::rpc::admin_cli::Destination::Stdout(),
                )?;
            }
            CmdReport::Delete(local_args) => {
                cli_output(
                    delete(cli.grpc_conn, local_args).await?,
                    &cli.args.format,
                    ::rpc::admin_cli::Destination::Stdout(),
                )?;
            }
            CmdReport::Promote(local_args) => {
                cli_output(
                    promote(cli.grpc_conn, local_args).await?,
                    &cli.args.format,
                    ::rpc::admin_cli::Destination::Stdout(),
                )?;
            }
            CmdReport::Revoke(local_args) => {
                cli_output(
                    revoke(cli.grpc_conn, local_args).await?,
                    &cli.args.format,
                    ::rpc::admin_cli::Destination::Stdout(),
                )?;
            }
            CmdReport::Show(selector) => match selector {
                ShowFor::Id(local_args) => {
                    cli_output(
                        show_for_id(cli.grpc_conn, local_args).await?,
                        &cli.args.format,
                        ::rpc::admin_cli::Destination::Stdout(),
                    )?;
                }
                ShowFor::Machine(local_args) => {
                    cli_output(
                        show_for_machine(cli.grpc_conn, local_args).await?,
                        &cli.args.format,
                        ::rpc::admin_cli::Destination::Stdout(),
                    )?;
                }
                ShowFor::All => cli_output(
                    show_all(cli.grpc_conn).await?,
                    &cli.args.format,
                    ::rpc::admin_cli::Destination::Stdout(),
                )?,
            },
            CmdReport::List(selector) => match selector {
                List::Machines(local_args) => {
                    cli_output(
                        list_machines(cli.grpc_conn, local_args).await?,
                        &cli.args.format,
                        ::rpc::admin_cli::Destination::Stdout(),
                    )?;
                }
                List::All(_) => {
                    cli_output(
                        list_all(cli.grpc_conn).await?,
                        &cli.args.format,
                        ::rpc::admin_cli::Destination::Stdout(),
                    )?;
                }
            },
            CmdReport::Match(local_args) => {
                cli_output(
                    match_values(cli.grpc_conn, local_args).await?,
                    &cli.args.format,
                    ::rpc::admin_cli::Destination::Stdout(),
                )?;
            }
        }
        Ok(())
    }

    /// create_for_id creates a new measurement report.
    pub async fn create_for_id(
        grpc_conn: &ApiClient,
        create: Create,
    ) -> CarbideCliResult<MeasurementReport> {
        // Request.
        let request = CreateMeasurementReportRequest {
            machine_id: create.machine_id.to_string(),
            pcr_values: create.values.into_iter().map(Into::into).collect(),
        };

        // Response.
        let response = grpc_conn.0.create_measurement_report(request).await?;

        MeasurementReport::from_grpc(response.report.as_ref())
            .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))
    }

    /// delete deletes a measurement report with the provided ID.
    pub async fn delete(
        grpc_conn: &ApiClient,
        delete: Delete,
    ) -> CarbideCliResult<MeasurementReport> {
        // Request.
        let request = DeleteMeasurementReportRequest {
            report_id: Some(delete.report_id),
        };

        // Response.
        let response = grpc_conn.0.delete_measurement_report(request).await?;

        MeasurementReport::from_grpc(response.report.as_ref())
            .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))
    }

    /// promote promotes a report to an active bundle.
    ///
    /// `report promote <report-id> [pcr-selector]`
    pub async fn promote(
        grpc_conn: &ApiClient,
        promote: Promote,
    ) -> CarbideCliResult<MeasurementBundle> {
        // Request.
        let request = PromoteMeasurementReportRequest {
            report_id: Some(promote.report_id),
            pcr_registers: match &promote.pcr_registers {
                None => "".to_string(),
                Some(pcr_set) => pcr_set.to_string(),
            },
        };

        // Response.
        let response = grpc_conn.0.promote_measurement_report(request).await?;

        MeasurementBundle::from_grpc(response.bundle.as_ref())
            .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))
    }

    /// revoke "promotes" a journal entry into a revoked bundle,
    /// which is a way of being able to say "any journals that come in
    /// matching this should be marked as rejected.
    ///
    /// `journal revoke <journal-id> [pcr-selector]`
    pub async fn revoke(
        grpc_conn: &ApiClient,
        revoke: Revoke,
    ) -> CarbideCliResult<MeasurementBundle> {
        // Request.
        let request = RevokeMeasurementReportRequest {
            report_id: Some(revoke.report_id),
            pcr_registers: match &revoke.pcr_registers {
                None => "".to_string(),
                Some(pcr_set) => pcr_set.to_string(),
            },
        };

        // Response.
        let response = grpc_conn.0.revoke_measurement_report(request).await?;

        MeasurementBundle::from_grpc(response.bundle.as_ref())
            .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))
    }

    /// show_for_id dumps all info about a report for the given ID.
    pub async fn show_for_id(
        grpc_conn: &ApiClient,
        show_for_id: ShowForId,
    ) -> CarbideCliResult<MeasurementReport> {
        // Request.
        let request = ShowMeasurementReportForIdRequest {
            report_id: Some(show_for_id.report_id),
        };

        // Response.
        let response = grpc_conn.0.show_measurement_report_for_id(request).await?;

        MeasurementReport::from_grpc(response.report.as_ref())
            .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))
    }

    /// show_for_machine dumps reports for a given machine.
    pub async fn show_for_machine(
        grpc_conn: &ApiClient,
        show_for_machine: ShowForMachine,
    ) -> CarbideCliResult<MeasurementReportList> {
        // Request.
        let request = ShowMeasurementReportsForMachineRequest {
            machine_id: show_for_machine.machine_id.to_string(),
        };

        // Response.
        Ok(MeasurementReportList(
            grpc_conn
                .0
                .show_measurement_reports_for_machine(request)
                .await?
                .reports
                .into_iter()
                .map(|report| {
                    MeasurementReport::try_from(report).map_err(|e| {
                        CarbideCliError::GenericError(format!("conversion failed: {e}"))
                    })
                })
                .collect::<CarbideCliResult<Vec<MeasurementReport>>>()?,
        ))
    }

    /// show_all dumps all info about all reports.
    pub async fn show_all(grpc_conn: &ApiClient) -> CarbideCliResult<MeasurementReportList> {
        Ok(MeasurementReportList(
            grpc_conn
                .0
                .show_measurement_reports()
                .await?
                .reports
                .into_iter()
                .map(|report| {
                    MeasurementReport::try_from(report).map_err(|e| {
                        CarbideCliError::GenericError(format!("conversion failed: {e}"))
                    })
                })
                .collect::<CarbideCliResult<Vec<MeasurementReport>>>()?,
        ))
    }

    /// list lists all bundle ids.
    pub async fn list_all(grpc_conn: &ApiClient) -> CarbideCliResult<MeasurementReportRecordList> {
        // Request.
        let request = ListMeasurementReportRequest { selector: None };

        // Response.
        Ok(MeasurementReportRecordList(
            grpc_conn
                .0
                .list_measurement_report(request)
                .await?
                .reports
                .into_iter()
                .map(|report| {
                    MeasurementReportRecord::try_from(report).map_err(|e| {
                        CarbideCliError::GenericError(format!("conversion failed: {e}"))
                    })
                })
                .collect::<CarbideCliResult<Vec<MeasurementReportRecord>>>()?,
        ))
    }

    /// list_machines lists all reports for the given machine ID.
    pub async fn list_machines(
        grpc_conn: &ApiClient,
        list_machines: ListMachines,
    ) -> CarbideCliResult<MeasurementReportRecordList> {
        // Request.
        let request = ListMeasurementReportRequest {
            selector: Some(list_measurement_report_request::Selector::MachineId(
                list_machines.machine_id.to_string(),
            )),
        };

        // Response.
        Ok(MeasurementReportRecordList(
            grpc_conn
                .0
                .list_measurement_report(request)
                .await?
                .reports
                .into_iter()
                .map(|report| {
                    MeasurementReportRecord::try_from(report).map_err(|e| {
                        CarbideCliError::GenericError(format!("conversion failed: {e}"))
                    })
                })
                .collect::<CarbideCliResult<Vec<MeasurementReportRecord>>>()?,
        ))
    }

    /// match_values matches all reports with the provided PCR values.
    ///
    /// `report match <pcr_register:val>,...`
    pub async fn match_values(
        grpc_conn: &ApiClient,
        match_args: Match,
    ) -> CarbideCliResult<MeasurementReportRecordList> {
        // Request.
        let request = MatchMeasurementReportRequest {
            pcr_values: match_args.values.into_iter().map(Into::into).collect(),
        };

        // Response.
        Ok(MeasurementReportRecordList(
            grpc_conn
                .0
                .match_measurement_report(request)
                .await?
                .reports
                .into_iter()
                .map(|report| {
                    MeasurementReportRecord::try_from(report).map_err(|e| {
                        CarbideCliError::GenericError(format!("conversion failed: {e}"))
                    })
                })
                .collect::<CarbideCliResult<Vec<MeasurementReportRecord>>>()?,
        ))
    }

    /// MeasurementReportRecordList just implements a newtype pattern
    /// for a Vec<MeasurementReportRecord> so the ToTable trait can
    /// be leveraged (since we don't define Vec).
    #[derive(Serialize)]
    pub struct MeasurementReportRecordList(Vec<MeasurementReportRecord>);

    impl ToTable for MeasurementReportRecordList {
        fn into_table(self) -> eyre::Result<String> {
            let mut table = prettytable::Table::new();
            table.add_row(prettytable::row!["report_id", "machine_id", "created_ts"]);
            for report in self.0.iter() {
                table.add_row(prettytable::row![
                    report.report_id,
                    report.machine_id,
                    report.ts
                ]);
            }
            Ok(table.to_string())
        }
    }

    /// MeasurementReportList just implements a newtype
    /// pattern for a Vec<MeasurementReport> so the ToTable
    /// trait can be leveraged (since we don't define Vec).
    #[derive(Serialize)]
    pub struct MeasurementReportList(Vec<MeasurementReport>);

    // When `report show` gets called (for all entries), and the output format
    // is the default table view, this gets used to print a pretty table.
    impl ToTable for MeasurementReportList {
        fn into_table(self) -> eyre::Result<String> {
            let mut table = prettytable::Table::new();
            table.add_row(prettytable::row!["report_id", "details", "values"]);
            for report in self.0.iter() {
                let mut details_table = prettytable::Table::new();
                details_table.add_row(prettytable::row!["report_id", report.report_id]);
                details_table.add_row(prettytable::row!["machine_id", report.machine_id]);
                details_table.add_row(prettytable::row!["created_ts", report.ts]);
                let mut values_table = prettytable::Table::new();
                values_table.add_row(prettytable::row!["pcr_register", "value"]);
                for value_record in report.values.iter() {
                    values_table.add_row(prettytable::row![
                        value_record.pcr_register,
                        value_record.sha_any
                    ]);
                }
                table.add_row(prettytable::row![
                    report.report_id,
                    details_table,
                    values_table
                ]);
            }
            Ok(table.to_string())
        }
    }
}
