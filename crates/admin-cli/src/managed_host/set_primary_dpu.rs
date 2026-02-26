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

    #[derive(Parser, Debug)]
    pub struct Args {
        #[clap(help = "ID of the host machine")]
        pub host_machine_id: MachineId,
        #[clap(help = "ID of the DPU machine to make primary")]
        pub dpu_machine_id: MachineId,
        #[clap(long, help = "Reboot the host after the update")]
        pub reboot: bool,
    }
}

pub mod cmd {
    use ::rpc::forge as forgerpc;

    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn set_primary_dpu(api_client: &ApiClient, args: Args) -> CarbideCliResult<()> {
        api_client
            .0
            .set_primary_dpu(forgerpc::SetPrimaryDpuRequest {
                host_machine_id: Some(args.host_machine_id),
                dpu_machine_id: Some(args.dpu_machine_id),
                reboot: args.reboot,
            })
            .await?;
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::set_primary_dpu(&ctx.api_client, self).await?;
        Ok(())
    }
}
