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

use crate::bmc_machine::common::InfiniteBootArgs;
use crate::cfg::run::Run;
use crate::cfg::runtime::RuntimeContext;

pub mod args {
    use clap::Parser;

    use super::*;

    // EnableInfiniteBootArgs wraps the shared InfiniteBootArgs as a subcommand
    // specific newtype to allow sharing of InfiniteBootArgs, and still
    // providing a subcommand-specific Run trait implementation.
    #[derive(Parser, Debug, Clone)]
    pub struct Args {
        #[clap(flatten)]
        pub inner: InfiniteBootArgs,
    }
}

pub mod cmd {
    use super::*;
    use crate::bmc_machine::common::AdminPowerControlAction;
    use crate::rpc::ApiClient;

    pub async fn enable_infinite_boot(
        args: InfiniteBootArgs,
        api_client: &ApiClient,
    ) -> CarbideCliResult<()> {
        let machine = args.machine;
        api_client
            .enable_infinite_boot(None, Some(machine.clone()))
            .await?;
        if args.reboot {
            api_client
                .admin_power_control(
                    None,
                    Some(machine),
                    AdminPowerControlAction::ForceRestart.into(),
                )
                .await?;
        }
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::enable_infinite_boot(self.inner, &ctx.api_client).await
    }
}
