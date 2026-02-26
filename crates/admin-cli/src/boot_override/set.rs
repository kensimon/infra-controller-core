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
    use carbide_uuid::machine::MachineInterfaceId;
    use clap::Parser;

    #[derive(Parser, Debug, Clone)]
    pub struct Args {
        pub interface_id: MachineInterfaceId,
        #[clap(short = 'p', long)]
        pub custom_pxe: Option<String>,
        #[clap(short = 'u', long)]
        pub custom_user_data: Option<String>,
    }
}

pub mod cmd {
    use std::path::PathBuf;

    use ::rpc::admin_cli::CarbideCliError;
    use ::rpc::forge::MachineBootOverride;

    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn set(args: Args, api_client: &ApiClient) -> CarbideCliResult<()> {
        if args.custom_pxe.is_none() && args.custom_user_data.is_none() {
            return Err(CarbideCliError::GenericError(
                "Either custom pxe or custom user data is required".to_owned(),
            ));
        }

        let custom_pxe_path = args.custom_pxe.map(PathBuf::from);
        let custom_user_data_path = args.custom_user_data.map(PathBuf::from);

        let custom_pxe = match &custom_pxe_path {
            Some(path) => Some(std::fs::read_to_string(path)?),
            None => None,
        };

        let custom_user_data = match &custom_user_data_path {
            Some(path) => Some(std::fs::read_to_string(path)?),
            None => None,
        };

        api_client
            .0
            .set_machine_boot_override(MachineBootOverride {
                machine_interface_id: Some(args.interface_id),
                custom_pxe,
                custom_user_data,
            })
            .await?;
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::set(self, &ctx.api_client).await
    }
}
