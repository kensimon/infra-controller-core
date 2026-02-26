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
    use clap::{ArgGroup, Parser};

    #[derive(Parser, Debug, Clone)]
    #[clap(group(ArgGroup::new("autoupdate_action").required(true).args(&["enable", "disable", "clear"])))]
    pub struct Args {
        #[clap(long, help = "Machine ID of the host to change")]
        pub machine: MachineId,
        #[clap(
            short = 'e',
            long,
            action,
            help = "Enable auto updates even if globally disabled or individually disabled by config files"
        )]
        pub enable: bool,
        #[clap(
            short = 'd',
            long,
            action,
            help = "Disable auto updates even if globally enabled or individually enabled by config files"
        )]
        pub disable: bool,
        #[clap(
            short = 'c',
            long,
            action,
            help = "Perform auto updates according to config files"
        )]
        pub clear: bool,
    }
}

pub mod cmd {
    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn autoupdate(cfg: Args, api_client: &ApiClient) -> CarbideCliResult<()> {
        let _response = api_client.machine_set_auto_update(cfg).await?;
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::autoupdate(self, &ctx.api_client).await?;
        Ok(())
    }
}
