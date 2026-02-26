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

use super::common::GlobalOptions;
use crate::cfg::run::Run;
use crate::cfg::runtime::RuntimeContext;

pub mod args {
    use carbide_uuid::instance::InstanceId;
    use clap::Parser;
    use rpc::forge::InstanceNvLinkConfig;

    #[derive(Parser, Debug)]
    pub struct Args {
        #[clap(short, long, required(true))]
        pub instance: InstanceId,
        #[clap(
            long,
            required(true),
            help = "NVLink configuration in JSON format",
            value_name = "NVLINK_JSON"
        )]
        pub config: InstanceNvLinkConfig,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::CarbideCliError;

    use super::args::Args;
    use super::*;
    use crate::instance::common::GlobalOptions;
    use crate::rpc::ApiClient;

    pub async fn update_nvlink_config(
        api_client: &ApiClient,
        update_request: Args,
        opts: &GlobalOptions<'_>,
    ) -> CarbideCliResult<()> {
        if opts.cloud_unsafe_op.is_none() {
            return Err(CarbideCliError::GenericError(
                "Operation not allowed due to potential inconsistencies with cloud database."
                    .to_owned(),
            ));
        }

        match api_client
            .update_instance_config_with(
                update_request.instance,
                |config| {
                    config.nvlink = Some(update_request.config.clone());
                },
                |_metadata| {},
                opts.cloud_unsafe_op.clone(),
            )
            .await
        {
            Ok(i) => {
                tracing::info!(
                    "update-nvlink-config was successful. Updated instance: {:?}",
                    i
                );
            }
            Err(e) => {
                tracing::info!("update-nvlink-config failed with {} ", e);
            }
        };
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        let opts = GlobalOptions {
            format: ctx.config.format,
            page_size: ctx.config.page_size,
            sort_by: &ctx.config.sort_by,
            cloud_unsafe_op: if ctx.config.cloud_unsafe_op_enabled {
                Some("enabled".to_string())
            } else {
                None
            },
        };
        cmd::update_nvlink_config(&ctx.api_client, self, &opts).await?;
        Ok(())
    }
}
