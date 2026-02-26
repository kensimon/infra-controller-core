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
    pub struct Args {
        #[clap(long, help = "ID of the machine to enable/disable lockdown")]
        pub machine: MachineId,
        #[clap(short, long, help = "Issue reboot to apply lockdown change")]
        pub reboot: bool,
        #[clap(
            long,
            conflicts_with = "disable",
            required_unless_present = "disable",
            help = "Enable lockdown"
        )]
        pub enable: bool,
        #[clap(
            long,
            conflicts_with = "enable",
            required_unless_present = "enable",
            help = "Disable lockdown"
        )]
        pub disable: bool,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::CarbideCliError;
    use ::rpc::forge as forgerpc;

    use super::args::Args;
    use super::*;
    use crate::bmc_machine::common::AdminPowerControlAction;
    use crate::rpc::ApiClient;

    pub async fn lockdown(args: Args, api_client: &ApiClient) -> CarbideCliResult<()> {
        let machine = args.machine;
        let action = if args.enable {
            forgerpc::LockdownAction::Enable
        } else if args.disable {
            forgerpc::LockdownAction::Disable
        } else {
            return Err(CarbideCliError::GenericError(
                "Either --enable or --disable must be specified".to_string(),
            ));
        };

        api_client.lockdown(None, machine, action).await?;

        let action_str = if args.enable { "enabled" } else { "disabled" };

        if args.reboot {
            api_client
                .admin_power_control(
                    None,
                    Some(machine.to_string()),
                    AdminPowerControlAction::ForceRestart.into(),
                )
                .await?;
            println!(
                "Lockdown {} and reboot initiated to apply the change.",
                action_str
            );
        } else {
            println!(
                "Lockdown {}. Please reboot the machine to apply the change.",
                action_str
            );
        }
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::lockdown(self, &ctx.api_client).await
    }
}
