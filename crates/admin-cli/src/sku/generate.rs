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
        #[clap(help = "The machine id of the machine to use to generate a SKU")]
        pub machine_id: MachineId,
        #[clap(help = "override the ID of the SKU", long)]
        pub id: Option<String>,
    }
}

pub mod cmd {
    use std::pin::Pin;

    use ::rpc::admin_cli::OutputFormat;

    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;
    use crate::sku::show::cmd::show_sku_details;

    pub async fn generate(
        args: Args,
        api_client: &ApiClient,
        output: &mut Pin<Box<dyn tokio::io::AsyncWrite>>,
        output_format: &OutputFormat,
        extended: bool,
    ) -> CarbideCliResult<()> {
        let mut sku = api_client
            .0
            .generate_sku_from_machine(args.machine_id)
            .await?;
        if let Some(id) = args.id {
            sku.id = id;
        }
        show_sku_details(output, output_format, extended, sku).await?;
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::generate(
            self,
            &ctx.api_client,
            &mut ctx.output_file,
            &ctx.config.format,
            ctx.config.extended,
        )
        .await
    }
}
