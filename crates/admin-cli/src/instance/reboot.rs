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
    use carbide_uuid::instance::InstanceId;
    use clap::Parser;

    #[derive(Parser, Debug)]
    pub struct Args {
        #[clap(short, long)]
        pub instance: InstanceId,

        #[clap(short, long, action)]
        pub custom_pxe: bool,

        #[clap(short, long, action)]
        pub apply_updates_on_reboot: bool,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::CarbideCliError;
    use ::rpc::forge::{
        InstancePowerRequest, {self as forgerpc},
    };

    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn handle_reboot(args: Args, api_client: &ApiClient) -> CarbideCliResult<()> {
        let machine_id = api_client
            .get_one_instance(args.instance)
            .await?
            .instances
            .last()
            .ok_or_else(|| CarbideCliError::GenericError("Unknown UUID".to_string()))?
            .machine_id
            .ok_or_else(|| {
                CarbideCliError::GenericError("Instance has no machine associated.".to_string())
            })?;

        api_client
            .0
            .invoke_instance_power(InstancePowerRequest {
                instance_id: Some(args.instance),
                machine_id: Some(machine_id),
                operation: forgerpc::instance_power_request::Operation::PowerReset as i32,
                boot_with_custom_ipxe: args.custom_pxe,
                apply_updates_on_reboot: args.apply_updates_on_reboot,
            })
            .await?;
        println!(
            "Reboot for instance {} (machine {}) is requested successfully!",
            args.instance, machine_id
        );

        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::handle_reboot(self, &ctx.api_client).await?;
        Ok(())
    }
}
