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
        #[clap(short = 'i', long = "id", help = "The extension service ID to delete")]
        pub service_id: String,

        #[clap(
            short = 'v',
            long,
            help = "Version strings to delete (optional, leave empty to keep all versions)",
            value_delimiter = ','
        )]
        pub versions: Vec<String>,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::output::OutputFormat;
    use ::rpc::forge::DeleteDpuExtensionServiceRequest;

    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn handle_delete(
        args: Args,
        _output_format: OutputFormat,
        api_client: &ApiClient,
    ) -> CarbideCliResult<()> {
        api_client
            .0
            .delete_dpu_extension_service(DeleteDpuExtensionServiceRequest {
                service_id: args.service_id,
                versions: args.versions,
            })
            .await?;

        println!("Delete successful");
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::handle_delete(self, ctx.config.format, &ctx.api_client).await
    }
}
