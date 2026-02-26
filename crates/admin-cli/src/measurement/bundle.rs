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

/**
 *  Measured Boot CLI arguments for the `measurement bundle` subcommand.
 *
 * This provides the CLI subcommands and arguments for:
 *  - `bundle create`: Create a new measurement bundle.
 *  - `bundle delete`: Delete an existing measurement bundle.
 *  - `bundle rename`: Rename an existing measurement bundle.
 *  - `bundle set-state`: Change the state of a measurement bundle.
 *  - `bundle show`: Show all details about measurement bundle(s).
 *  - `bundle list all`: List high level metadata about all bundles.
 *  - `bundle list machines`: List all matchines matching a given bundle.
*/
pub mod args {
    use carbide_uuid::measured_boot::{
        MeasurementBundleId, MeasurementReportId, MeasurementSystemProfileId,
    };
    use clap::Parser;
    use measured_boot::pcr::PcrRegisterValue;
    use measured_boot::records::MeasurementBundleState;

    use crate::cfg::measurement::parse_pcr_register_values;
    use crate::measurement::global::cmds::IdNameIdentifier;

    /// CmdBundle provides a container for the `bundle` subcommand, which itself
    /// contains other subcommands for working with profiles.
    #[derive(Parser, Debug)]
    pub enum CmdBundle {
        #[clap(
            about = "Create a new bundle with a given values, for a given profile ID.",
            visible_alias = "c"
        )]
        Create(Create),

        #[clap(about = "Delete a bundle based on ID", visible_alias = "d")]
        Delete(Delete),

        #[clap(about = "Rename a bundle.", visible_alias = "r")]
        Rename(Rename),

        #[clap(about = "Set a new state for a bundle.", visible_alias = "u")]
        SetState(SetState),

        #[clap(about = "Show a bundle (or all).", visible_alias = "s")]
        Show(Show),

        #[clap(
            subcommand,
            about = "Get closest bundle to a report.",
            visible_alias = "g"
        )]
        FindClosestMatch(FindClosestMatch),

        #[clap(
            subcommand,
            about = "List bundles by various ways.",
            visible_alias = "l"
        )]
        List(List),
    }

    /// Create is used to create a new bundle, associated with a given profile ID
    /// or profile name, with provided PCR values and an optional
    /// MeasurementBundleState (the default is 'active').
    #[derive(Parser, Debug)]
    pub struct Create {
        #[clap(help = "A human-readable name to give this bundle.")]
        pub name: String,

        #[clap(help = "The profile ID of the profile to associate this bundle with.")]
        pub profile_id: MeasurementSystemProfileId,

        #[clap(
            required = true,
            use_value_delimiter = true,
            value_delimiter = ',',
            help = "Comma-separated list of {pcr_register:value,...} to associate with this bundle."
        )]
        #[arg(value_parser = parse_pcr_register_values)]
        pub values: Vec<PcrRegisterValue>,

        // state is optional, and if unset, the database itself
        // is configured to default to 'active'.
        #[clap(
            long,
            value_enum,
            help = "The state for this bundle (default: active)."
        )]
        pub state: Option<MeasurementBundleState>,
    }

    /// Delete will delete a bundle for the given ID.
    #[derive(Parser, Debug)]
    pub struct Delete {
        #[clap(help = "The bundle ID.")]
        pub bundle_id: MeasurementBundleId,

        #[clap(long, help = "Also purge any journal records for this bundle.")]
        pub purge_journals: bool,
    }

    /// Rename will rename a bundle for the given ID or name.
    /// A parser will parse the `identifier` to determine if
    /// the API should be called w/ an ID or name selector.
    #[derive(Parser, Debug)]
    pub struct Rename {
        #[clap(help = "The existing bundle ID or name.")]
        pub identifier: String,

        #[clap(help = "The new bundle name.")]
        pub new_bundle_name: String,

        #[clap(long, help = "Explicitly say the identifier is bundle ID.")]
        pub is_id: bool,

        #[clap(long, help = "Explicitly say the identifier is a bundle name.")]
        pub is_name: bool,
    }

    impl IdNameIdentifier for Rename {
        fn is_id(&self) -> bool {
            self.is_id
        }

        fn is_name(&self) -> bool {
            self.is_name
        }
    }

    /// Show will get + display a bundle for the given ID, or, if not ID is set,
    /// it will display all bundles and their information.
    #[derive(Parser, Debug)]
    pub struct Show {
        #[clap(help = "The optional bundle ID or name.")]
        pub identifier: Option<String>,

        #[clap(long, help = "Explicitly say the identifier is bundle ID.")]
        pub is_id: bool,

        #[clap(long, help = "Explicitly say the identifier is a bundle name.")]
        pub is_name: bool,
    }

    impl IdNameIdentifier for Show {
        fn is_id(&self) -> bool {
            self.is_id
        }

        fn is_name(&self) -> bool {
            self.is_name
        }
    }

    /// SetState is used to set the state of the bundle (e.g. active, obsolete,
    /// retired, revoked).
    #[derive(Parser, Debug)]
    pub struct SetState {
        #[clap(help = "The bundle ID or name to update.")]
        pub identifier: String,

        #[clap(
            required = true,
            value_enum,
            help = "The state to set for this bundle."
        )]
        pub state: MeasurementBundleState,

        #[clap(long, help = "Explicitly say the identifier is bundle ID.")]
        pub is_id: bool,

        #[clap(long, help = "Explicitly say the identifier is a bundle name.")]
        pub is_name: bool,
    }

    impl IdNameIdentifier for SetState {
        fn is_id(&self) -> bool {
            self.is_id
        }

        fn is_name(&self) -> bool {
            self.is_name
        }
    }

    /// List provides a few ways to list things.
    #[derive(Parser, Debug)]
    pub enum List {
        #[clap(about = "List all bundles", visible_alias = "a")]
        All(ListAll),

        #[clap(
            about = "List all machines for a given bundle ID.",
            visible_alias = "m"
        )]
        Machines(ListMachines),
    }

    /// ListAll will list all bundles.
    #[derive(Parser, Debug)]
    pub struct ListAll {}

    /// ListMachines lists all machines for a given bundle (by bundle name or ID).
    #[derive(Parser, Debug)]
    pub struct ListMachines {
        #[clap(help = "The existing bundle ID or name.")]
        pub identifier: String,

        #[clap(long, help = "Explicitly say the identifier is bundle ID.")]
        pub is_id: bool,

        #[clap(long, help = "Explicitly say the identifier is a bundle name.")]
        pub is_name: bool,
    }

    impl IdNameIdentifier for ListMachines {
        fn is_id(&self) -> bool {
            self.is_id
        }

        fn is_name(&self) -> bool {
            self.is_name
        }
    }

    #[derive(Parser, Debug)]
    pub enum FindClosestMatch {
        #[clap(about = "The existing report ID.")]
        Report(ReportId),
    }

    #[derive(Parser, Debug)]
    pub struct ReportId {
        #[clap(help = "Report ID.")]
        pub id: MeasurementReportId,
    }
}

/// `measurement bundle` subcommand dispatcher + backing functions.
pub mod cmds {
    use std::str::FromStr;

    use ::rpc::admin_cli::{CarbideCliError, CarbideCliResult, ToTable, cli_output};
    use ::rpc::protos::measured_boot::{
        CreateMeasurementBundleRequest, DeleteMeasurementBundleRequest,
        FindClosestBundleMatchRequest, ListMeasurementBundleMachinesRequest,
        MeasurementBundleStatePb, RenameMeasurementBundleRequest, ShowMeasurementBundleRequest,
        UpdateMeasurementBundleRequest, delete_measurement_bundle_request,
        list_measurement_bundle_machines_request, rename_measurement_bundle_request,
        show_measurement_bundle_request, update_measurement_bundle_request,
    };
    use carbide_uuid::machine::MachineId;
    use carbide_uuid::measured_boot::MeasurementBundleId;
    use measured_boot::bundle::MeasurementBundle;
    use measured_boot::records::MeasurementBundleRecord;
    use serde::Serialize;

    use crate::measurement::bundle::args::{
        CmdBundle, Create, Delete, FindClosestMatch, List, ListMachines, Rename, SetState, Show,
    };
    use crate::measurement::global::cmds::{IdentifierType, get_identifier};
    use crate::measurement::{MachineIdList, global};
    use crate::rpc::ApiClient;

    /// dispatch matches + dispatches the correct command for
    /// the `bundle` subcommand (e.g. create, delete, set-state).
    pub async fn dispatch(
        cmd: CmdBundle,
        cli: &mut global::cmds::CliData<'_, '_>,
    ) -> CarbideCliResult<()> {
        match cmd {
            CmdBundle::Create(local_args) => {
                cli_output(
                    create_for_id(cli.grpc_conn, local_args).await?,
                    &cli.args.format,
                    ::rpc::admin_cli::Destination::Stdout(),
                )?;
            }
            CmdBundle::Delete(local_args) => {
                cli_output(
                    delete(cli.grpc_conn, local_args).await?,
                    &cli.args.format,
                    ::rpc::admin_cli::Destination::Stdout(),
                )?;
            }
            CmdBundle::Rename(local_args) => {
                cli_output(
                    rename(cli.grpc_conn, local_args).await?,
                    &cli.args.format,
                    ::rpc::admin_cli::Destination::Stdout(),
                )?;
            }
            CmdBundle::SetState(local_args) => {
                cli_output(
                    set_state(cli.grpc_conn, local_args).await?,
                    &cli.args.format,
                    ::rpc::admin_cli::Destination::Stdout(),
                )?;
            }
            CmdBundle::Show(local_args) => {
                if local_args.identifier.is_some() {
                    cli_output(
                        show_by_id_or_name(cli.grpc_conn, local_args).await?,
                        &cli.args.format,
                        ::rpc::admin_cli::Destination::Stdout(),
                    )?;
                } else {
                    cli_output(
                        show_all(cli.grpc_conn, local_args).await?,
                        &cli.args.format,
                        ::rpc::admin_cli::Destination::Stdout(),
                    )?;
                }
            }
            CmdBundle::FindClosestMatch(local_args) => {
                match find_closest_match(cli.grpc_conn, local_args).await? {
                    Some(measurement_bundle) => cli_output(
                        measurement_bundle,
                        &cli.args.format,
                        ::rpc::admin_cli::Destination::Stdout(),
                    )?,
                    None => tracing::info!("No partially matching bundle found"),
                };
            }
            CmdBundle::List(selector) => match selector {
                List::Machines(local_args) => {
                    cli_output(
                        list_machines(cli.grpc_conn, local_args).await?,
                        &cli.args.format,
                        ::rpc::admin_cli::Destination::Stdout(),
                    )?;
                }
                List::All(_) => {
                    cli_output(
                        list(cli.grpc_conn).await?,
                        &cli.args.format,
                        ::rpc::admin_cli::Destination::Stdout(),
                    )?;
                }
            },
        }
        Ok(())
    }

    /// create_for_id creates a new measurement bundle associated with the
    /// profile w/ the provided profile ID.
    pub async fn create_for_id(
        grpc_conn: &ApiClient,
        create: Create,
    ) -> CarbideCliResult<MeasurementBundle> {
        // Prepare.
        let state: MeasurementBundleStatePb = match create.state {
            Some(input_state) => input_state.into(),
            None => MeasurementBundleStatePb::Active,
        };

        // Request.
        let request = CreateMeasurementBundleRequest {
            name: Some(create.name),
            profile_id: Some(create.profile_id),
            pcr_values: create.values.into_iter().map(Into::into).collect(),
            state: state.into(),
        };

        // Response.
        let response = grpc_conn.0.create_measurement_bundle(request).await?;

        MeasurementBundle::from_grpc(response.bundle.as_ref())
            .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))
    }

    /// delete deletes a measurement bundle with the provided ID.
    pub async fn delete(
        grpc_conn: &ApiClient,
        delete: Delete,
    ) -> CarbideCliResult<MeasurementBundle> {
        // Request.
        let request = DeleteMeasurementBundleRequest {
            selector: Some(delete_measurement_bundle_request::Selector::BundleId(
                delete.bundle_id,
            )),
        };

        // Response.
        let response = grpc_conn.0.delete_measurement_bundle(request).await?;

        MeasurementBundle::from_grpc(response.bundle.as_ref())
            .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))
    }

    /// rename renames a measurement bundle with the provided name or ID.
    pub async fn rename(
        grpc_conn: &ApiClient,
        rename: Rename,
    ) -> CarbideCliResult<MeasurementBundle> {
        // Prepare.
        let selector = match get_identifier(&rename)? {
            IdentifierType::ForId => {
                let bundle_id = MeasurementBundleId::from_str(&rename.identifier)
                    .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))?;
                Some(rename_measurement_bundle_request::Selector::BundleId(
                    bundle_id,
                ))
            }
            IdentifierType::ForName => Some(
                rename_measurement_bundle_request::Selector::BundleName(rename.identifier),
            ),
            IdentifierType::Detect => match MeasurementBundleId::from_str(&rename.identifier) {
                Ok(bundle_id) => Some(rename_measurement_bundle_request::Selector::BundleId(
                    bundle_id,
                )),
                Err(_) => Some(rename_measurement_bundle_request::Selector::BundleName(
                    rename.identifier,
                )),
            },
        };

        // Request.
        let request = RenameMeasurementBundleRequest {
            new_bundle_name: rename.new_bundle_name,
            selector,
        };

        // Response.
        let response = grpc_conn.0.rename_measurement_bundle(request).await?;

        MeasurementBundle::from_grpc(response.bundle.as_ref())
            .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))
    }

    /// set_state updates the state of the bundle (e.g. active, obsolete, retired).
    pub async fn set_state(
        grpc_conn: &ApiClient,
        set_state: SetState,
    ) -> CarbideCliResult<MeasurementBundle> {
        // Prepare.
        let state: MeasurementBundleStatePb = set_state.state.into();

        let selector = match get_identifier(&set_state)? {
            IdentifierType::ForId => {
                let bundle_id = MeasurementBundleId::from_str(&set_state.identifier)
                    .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))?;
                Some(update_measurement_bundle_request::Selector::BundleId(
                    bundle_id,
                ))
            }
            IdentifierType::ForName => Some(
                update_measurement_bundle_request::Selector::BundleName(set_state.identifier),
            ),
            IdentifierType::Detect => match MeasurementBundleId::from_str(&set_state.identifier) {
                Ok(bundle_id) => Some(update_measurement_bundle_request::Selector::BundleId(
                    bundle_id,
                )),
                Err(_) => Some(update_measurement_bundle_request::Selector::BundleName(
                    set_state.identifier,
                )),
            },
        };

        // Request.
        let request = UpdateMeasurementBundleRequest {
            state: state.into(),
            selector,
        };

        // Response.
        let response = grpc_conn.0.update_measurement_bundle(request).await?;

        MeasurementBundle::from_grpc(response.bundle.as_ref())
            .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))
    }

    /// show_by_id dumps all info about a bundle for the given ID or name.
    pub async fn show_by_id_or_name(
        grpc_conn: &ApiClient,
        show: Show,
    ) -> CarbideCliResult<MeasurementBundle> {
        let identifier_type = get_identifier(&show)?;
        // Prepare.
        let identifier = show
            .identifier
            .ok_or(CarbideCliError::GenericError(String::from(
                "identifier expected to be set here",
            )))?;

        let selector = match identifier_type {
            IdentifierType::ForId => {
                let bundle_id = MeasurementBundleId::from_str(&identifier)
                    .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))?;
                Some(show_measurement_bundle_request::Selector::BundleId(
                    bundle_id,
                ))
            }
            IdentifierType::ForName => Some(show_measurement_bundle_request::Selector::BundleName(
                identifier,
            )),
            IdentifierType::Detect => match MeasurementBundleId::from_str(&identifier) {
                Ok(bundle_id) => Some(show_measurement_bundle_request::Selector::BundleId(
                    bundle_id,
                )),
                Err(_) => Some(show_measurement_bundle_request::Selector::BundleName(
                    identifier,
                )),
            },
        };

        // Request.
        let request = ShowMeasurementBundleRequest { selector };

        // Response.
        let response = grpc_conn.0.show_measurement_bundle(request).await?;

        MeasurementBundle::from_grpc(response.bundle.as_ref())
            .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))
    }

    /// show_all dumps all info about all bundles.
    pub async fn show_all(
        grpc_conn: &ApiClient,
        _get_by_id: Show,
    ) -> CarbideCliResult<MeasurementBundleList> {
        Ok(MeasurementBundleList(
            grpc_conn
                .0
                .show_measurement_bundles()
                .await?
                .bundles
                .into_iter()
                .map(|bundle| {
                    MeasurementBundle::try_from(bundle).map_err(|e| {
                        CarbideCliError::GenericError(format!("conversion failed: {e}"))
                    })
                })
                .collect::<CarbideCliResult<Vec<MeasurementBundle>>>()?,
        ))
    }

    /// list lists all bundle ids.
    pub async fn list(grpc_conn: &ApiClient) -> CarbideCliResult<MeasurementBundleRecordList> {
        Ok(MeasurementBundleRecordList(
            grpc_conn
                .0
                .list_measurement_bundles()
                .await?
                .bundles
                .into_iter()
                .map(|rec| {
                    MeasurementBundleRecord::try_from(rec).map_err(|e| {
                        CarbideCliError::GenericError(format!("conversion failed: {e}"))
                    })
                })
                .collect::<CarbideCliResult<Vec<MeasurementBundleRecord>>>()?,
        ))
    }

    /// list_machines lists all machines associated with the provided
    /// bundle ID or bundle name.
    pub async fn list_machines(
        grpc_conn: &ApiClient,
        list_machines: ListMachines,
    ) -> CarbideCliResult<MachineIdList> {
        // Prepare.
        let selector = match get_identifier(&list_machines)? {
            IdentifierType::ForId => {
                let bundle_id = MeasurementBundleId::from_str(&list_machines.identifier)
                    .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))?;
                Some(list_measurement_bundle_machines_request::Selector::BundleId(bundle_id))
            }
            IdentifierType::ForName => Some(
                list_measurement_bundle_machines_request::Selector::BundleName(
                    list_machines.identifier,
                ),
            ),
            IdentifierType::Detect => {
                match MeasurementBundleId::from_str(&list_machines.identifier) {
                    Ok(bundle_id) => Some(
                        list_measurement_bundle_machines_request::Selector::BundleId(bundle_id),
                    ),
                    Err(_) => Some(
                        list_measurement_bundle_machines_request::Selector::BundleName(
                            list_machines.identifier,
                        ),
                    ),
                }
            }
        };

        // Request.
        let request = ListMeasurementBundleMachinesRequest { selector };

        // Response.
        Ok(MachineIdList(
            grpc_conn
                .0
                .list_measurement_bundle_machines(request)
                .await?
                .machine_ids
                .iter()
                .map(|rec| {
                    MachineId::from_str(rec).map_err(|e| {
                        CarbideCliError::GenericError(format!("conversion failed: {e}"))
                    })
                })
                .collect::<CarbideCliResult<Vec<MachineId>>>()?,
        ))
    }

    pub async fn find_closest_match(
        grpc_conn: &ApiClient,
        args: FindClosestMatch,
    ) -> CarbideCliResult<Option<MeasurementBundle>> {
        // At the moment, the request only contains report id
        // but this can be expanded to contain journal id also
        let request = match args {
            FindClosestMatch::Report(report_id) => FindClosestBundleMatchRequest {
                report_id: Some(report_id.id),
            },
        };

        // Response.
        let response = grpc_conn.0.find_closest_bundle_match(request).await?;

        if response.bundle.is_none() {
            return Ok(None);
        }

        Ok(Some(
            MeasurementBundle::from_grpc(response.bundle.as_ref())
                .map_err(|e| crate::CarbideCliError::GenericError(e.to_string()))?,
        ))
    }

    /// MeasurementBundleRecordList just implements a newtype pattern
    /// for a Vec<MeasurementBundleRecord> so the ToTable trait can
    /// be leveraged (since we don't define Vec).
    #[derive(Serialize)]
    pub struct MeasurementBundleRecordList(Vec<MeasurementBundleRecord>);

    impl ToTable for MeasurementBundleRecordList {
        fn into_table(self) -> eyre::Result<String> {
            let mut table = prettytable::Table::new();
            table.add_row(prettytable::row![
                Fg->"bundle_id",
                Fg->"profile_id",
                Fg->"name",
                Fg->"state",
                Fg->"created_ts"
            ]);
            for bundle in self.0.iter() {
                table.add_row(prettytable::row![
                    bundle.bundle_id,
                    bundle.profile_id,
                    bundle.name,
                    bundle.state,
                    bundle.ts
                ]);
            }
            Ok(table.to_string())
        }
    }

    /// MeasurementBundleList just implements a newtype
    /// pattern for a Vec<MeasurementBundle> so the ToTable
    /// trait can be leveraged (since we don't define Vec).
    #[derive(Serialize)]
    pub struct MeasurementBundleList(Vec<MeasurementBundle>);

    // When `bundle show` gets called (for all entries), and the output format
    // is the default table view, this gets used to print a pretty table.
    impl ToTable for MeasurementBundleList {
        fn into_table(self) -> eyre::Result<String> {
            let mut table = prettytable::Table::new();
            table.add_row(prettytable::row!["bundle_id", "details", "values"]);
            for bundle in self.0.iter() {
                let mut details_table = prettytable::Table::new();
                details_table.add_row(prettytable::row!["profile_id", bundle.profile_id]);
                details_table.add_row(prettytable::row!["name", bundle.name]);
                details_table.add_row(prettytable::row!["state", bundle.state]);
                details_table.add_row(prettytable::row!["created_ts", bundle.ts]);
                let mut values_table = prettytable::Table::new();
                values_table.add_row(prettytable::row!["pcr_register", "value"]);
                for value_record in bundle.values.iter() {
                    values_table.add_row(prettytable::row![
                        value_record.pcr_register,
                        value_record.sha_any
                    ]);
                }
                table.add_row(prettytable::row![
                    bundle.bundle_id,
                    details_table,
                    values_table
                ]);
            }
            Ok(table.to_string())
        }
    }
}
