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
    use clap::Parser;

    #[derive(Parser, Debug)]
    pub struct Args {
        #[clap(long, help = "ID of the machine to reboot")]
        pub machine: String,
    }
}

pub mod cmd {
    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn reboot(api_client: &ApiClient, args: Args) -> CarbideCliResult<()> {
        let res = api_client
            .admin_power_control(
                None,
                Some(args.machine),
                ::rpc::forge::admin_power_control_request::SystemPowerControl::ForceRestart,
            )
            .await?;

        if let Some(msg) = res.msg {
            println!("{msg}");
        }
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::reboot(&ctx.api_client, self).await?;
        Ok(())
    }
}
