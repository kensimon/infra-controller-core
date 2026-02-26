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

    #[derive(Parser, Debug, Clone)]
    pub struct Args {
        #[clap(short = 'i', long, help = "uuid of the OS image to update.")]
        pub id: String,
        #[clap(short = 'n', long, help = "Optional, name of the OS image entry.")]
        pub name: Option<String>,
        #[clap(
            short = 'd',
            long,
            help = "Optional, description of the OS image entry."
        )]
        pub description: Option<String>,
        #[clap(
            short = 'y',
            long,
            help = "Optional, Authentication type, usually Bearer."
        )]
        pub auth_type: Option<String>,
        #[clap(
            short = 'p',
            long,
            help = "Optional, Authentication token, usually in base64."
        )]
        pub auth_token: Option<String>,
    }
}

pub mod cmd {
    use super::args::Args;
    use super::*;
    use crate::os_image::common::str_to_rpc_uuid;
    use crate::rpc::ApiClient;

    pub async fn update(args: Args, api_client: &ApiClient) -> CarbideCliResult<()> {
        let id = str_to_rpc_uuid(&args.id)?;
        let image = api_client
            .update_os_image(
                id,
                args.auth_type,
                args.auth_token,
                args.name,
                args.description,
            )
            .await?;
        if let Some(x) = image.attributes {
            if let Some(y) = x.id {
                println!("OS image {y} updated successfully.");
            } else {
                eprintln!("Updating the OS image may have failed, image id missing.");
            }
        } else {
            eprintln!("Updating the OS image may have failed, image attributes missing.");
        }
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::update(self, &ctx.api_client).await
    }
}
