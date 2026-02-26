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
        #[clap(long, help = "ID of the machine to check lockdown status")]
        pub machine: MachineId,
    }
}

pub mod cmd {
    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn lockdown_status(args: Args, api_client: &ApiClient) -> CarbideCliResult<()> {
        let response = api_client.lockdown_status(None, args.machine).await?;
        // Convert status enum to string
        let status_str = match response.status {
            0 => "Enabled",  // InternalLockdownStatus::ENABLED
            1 => "Partial",  // InternalLockdownStatus::PARTIAL
            2 => "Disabled", // InternalLockdownStatus::DISABLED
            _ => "Unknown",
        };
        println!("{}: {}", status_str, response.message);
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::lockdown_status(self, &ctx.api_client).await
    }
}
