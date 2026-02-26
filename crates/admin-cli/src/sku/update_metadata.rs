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
    use clap::{ArgGroup, Parser};

    #[derive(Parser, Debug)]
    #[clap(group(ArgGroup::new("group").required(true).multiple(true).args(&["description", "device_type"])))]
    pub struct Args {
        #[clap(help = "SKU ID of the SKU to update")]
        pub sku_id: String,
        #[clap(help = "Update the SKU's description", long, group("group"))]
        pub description: Option<String>,
        #[clap(help = "Update the SKU's device type", long, group("group"))]
        pub device_type: Option<String>,
    }

    impl From<Args> for ::rpc::forge::SkuUpdateMetadataRequest {
        fn from(value: Args) -> Self {
            ::rpc::forge::SkuUpdateMetadataRequest {
                sku_id: value.sku_id,
                description: value.description,
                device_type: value.device_type,
            }
        }
    }
}

pub mod cmd {
    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn update_metadata(args: Args, api_client: &ApiClient) -> CarbideCliResult<()> {
        api_client.0.update_sku_metadata(args).await?;
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::update_metadata(self, &ctx.api_client).await
    }
}
