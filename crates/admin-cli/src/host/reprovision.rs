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
use carbide_uuid::machine::MachineId;

use crate::cfg::run::Run;
use crate::cfg::runtime::RuntimeContext;

pub mod args {
    use clap::Parser;

    use super::*;

    #[derive(Parser, Debug, Clone)]
    pub enum Args {
        #[clap(about = "Set the host in reprovisioning mode.")]
        Set(ReprovisionSet),
        #[clap(about = "Clear the reprovisioning mode.")]
        Clear(ReprovisionClear),
        #[clap(about = "List all hosts pending reprovisioning.")]
        List,
        // TODO: Remove when manual upgrade feature is removed
        #[clap(about = "Mark manual firmware upgrade as complete for a host.")]
        MarkManualUpgradeComplete(ManualFirmwareUpgradeComplete),
    }

    #[derive(Parser, Debug, Clone)]
    pub struct ReprovisionSet {
        #[clap(short, long, help = "Machine ID for which reprovisioning is needed.")]
        pub id: MachineId,

        #[clap(short, long, action)]
        pub update_firmware: bool,

        #[clap(
            long,
            alias = "maintenance_reference",
            help = "If set, a HostUpdateInProgress health alert will be applied to the host"
        )]
        pub update_message: Option<String>,
    }

    #[derive(Parser, Debug, Clone)]
    pub struct ReprovisionClear {
        #[clap(
            short,
            long,
            help = "Machine ID for which reprovisioning should be cleared."
        )]
        pub id: MachineId,

        #[clap(short, long, action)]
        pub update_firmware: bool,
    }

    #[derive(Parser, Debug, Clone)]
    pub struct ManualFirmwareUpgradeComplete {
        #[clap(
            short,
            long,
            help = "Machine ID for which manual firmware upgrade should be set."
        )]
        pub id: MachineId,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::CarbideCliError;
    use ::rpc::forge::host_reprovisioning_request::Mode;
    use ::rpc::forge::{HostReprovisioningRequest, UpdateInitiator};
    use prettytable::{Table, row};

    use super::*;
    use crate::machine::{HealthOverrideTemplates, get_health_report};
    use crate::rpc::ApiClient;

    pub async fn trigger_reprovisioning(
        host_id: MachineId,
        mode: Mode,
        api_client: &ApiClient,
        update_message: Option<String>,
    ) -> CarbideCliResult<()> {
        if let (Mode::Set, Some(update_message)) = (mode, update_message) {
            // Set a HostUpdateInProgress health override on the Host

            let host_machine = api_client
                .get_machines_by_ids(&[host_id])
                .await?
                .machines
                .into_iter()
                .next();

            if let Some(host_machine) = host_machine
                && host_machine
                    .health_overrides
                    .iter()
                    .any(|or| or.source == "host-update")
            {
                return Err(CarbideCliError::GenericError(format!(
                    "Host machine: {:?} already has a \"host-update\" override.",
                    host_machine.id,
                )));
            }

            let report =
                get_health_report(HealthOverrideTemplates::HostUpdate, Some(update_message));

            api_client
                .machine_insert_health_report_override(host_id, report.into(), false)
                .await?;
        }
        api_client
            .0
            .trigger_host_reprovisioning(HostReprovisioningRequest {
                machine_id: Some(host_id),
                mode: mode as i32,
                initiator: UpdateInitiator::AdminCli as i32,
            })
            .await?;

        Ok(())
    }

    pub async fn list_hosts_pending(api_client: &ApiClient) -> CarbideCliResult<()> {
        let response = api_client.0.list_hosts_waiting_for_reprovisioning().await?;
        print_pending_hosts(response);
        Ok(())
    }

    pub async fn mark_manual_firmware_upgrade_complete(
        machine_id: MachineId,
        api_client: &ApiClient,
    ) -> CarbideCliResult<()> {
        api_client
            .0
            .mark_manual_firmware_upgrade_complete(machine_id)
            .await?;

        println!("Marked manual firmware upgrade as complete for machine {machine_id}",);

        Ok(())
    }

    fn print_pending_hosts(hosts: ::rpc::forge::HostReprovisioningListResponse) {
        let mut table = Table::new();

        table.set_titles(row![
            "Id",
            "State",
            "Initiator",
            "Requested At",
            "Initiated At",
            "User Approved"
        ]);

        for host in hosts.hosts {
            let user_approval = if host.user_approval_received {
                "Yes"
            } else if host.state.contains("Assigned") {
                "No"
            } else {
                "NA"
            };
            table.add_row(row![
                host.id.unwrap_or_default().to_string(),
                host.state,
                host.initiator,
                host.requested_at.unwrap_or_default(),
                host.initiated_at
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| "Not Started".to_string()),
                user_approval
            ]);
        }

        table.printstd();
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        match self {
            Args::Set(data) => {
                cmd::trigger_reprovisioning(
                    data.id,
                    ::rpc::forge::host_reprovisioning_request::Mode::Set,
                    &ctx.api_client,
                    data.update_message,
                )
                .await
            }
            Args::Clear(data) => {
                cmd::trigger_reprovisioning(
                    data.id,
                    ::rpc::forge::host_reprovisioning_request::Mode::Clear,
                    &ctx.api_client,
                    None,
                )
                .await
            }
            Args::List => cmd::list_hosts_pending(&ctx.api_client).await,
            Args::MarkManualUpgradeComplete(data) => {
                cmd::mark_manual_firmware_upgrade_complete(data.id, &ctx.api_client).await
            }
        }
    }
}
