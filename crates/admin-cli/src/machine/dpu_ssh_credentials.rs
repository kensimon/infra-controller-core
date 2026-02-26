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

use super::common::MachineQuery;
use crate::cfg::run::Run;
use crate::cfg::runtime::RuntimeContext;

pub mod args {
    use clap::Parser;

    use super::*;

    // Args wraps the shared MachineQuery as a subcommand
    // specific newtype to allow sharing of MachineQuery, and still
    // providing a subcommand-specific Run trait implementation.
    #[derive(Parser, Debug, Clone)]
    pub struct Args {
        #[clap(flatten)]
        pub inner: MachineQuery,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::OutputFormat;

    use super::*;
    use crate::rpc::ApiClient;

    pub async fn dpu_ssh_credentials(
        api_client: &ApiClient,
        query: MachineQuery,
        format: OutputFormat,
    ) -> CarbideCliResult<()> {
        let cred = api_client
            .0
            .get_dpu_ssh_credential(query.query.to_string())
            .await?;
        if format == OutputFormat::Json {
            println!("{}", serde_json::to_string_pretty(&cred)?);
        } else {
            println!("{}:{}", cred.username, cred.password);
        }
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::dpu_ssh_credentials(&ctx.api_client, self.inner, ctx.config.format).await?;
        Ok(())
    }
}
