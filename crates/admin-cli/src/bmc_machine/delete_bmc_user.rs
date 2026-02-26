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
    use mac_address::MacAddress;

    #[derive(Parser, Debug, Clone)]
    pub struct Args {
        #[clap(long, short, help = "IP of the BMC where we want to delete a user")]
        pub ip_address: Option<String>,
        #[clap(long, help = "MAC of the BMC where we want to delete a user")]
        pub mac_address: Option<MacAddress>,
        #[clap(long, short, help = "ID of the machine where we want to delete a user")]
        pub machine: Option<String>,

        #[clap(long, short, help = "Username of BMC account to delete")]
        pub username: String,
    }
}

pub mod cmd {
    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn delete_bmc_user(args: Args, api_client: &ApiClient) -> CarbideCliResult<()> {
        api_client
            .delete_bmc_user(
                args.ip_address,
                args.mac_address,
                args.machine,
                args.username,
            )
            .await?;
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::delete_bmc_user(self, &ctx.api_client).await
    }
}
