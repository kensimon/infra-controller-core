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
use crate::dpf::common::DpfQuery;

pub mod args {
    use clap::Parser;

    use super::*;

    // Args wraps the shared DpfQuery as a subcommand
    // specific newtype to allow sharing of DpfQuery, and still
    // providing a subcommand-specific Run trait implementation.
    #[derive(Parser, Debug)]
    pub struct Args {
        #[clap(flatten)]
        pub inner: DpfQuery,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::OutputFormat;
    use rpc::admin_cli::CarbideCliError;

    use super::*;
    use crate::rpc::ApiClient;

    pub async fn modify_dpf_state(
        query: &DpfQuery,
        _format: OutputFormat, // TODO: Implement json output handling.
        api_client: &ApiClient,
        enabled: bool,
    ) -> CarbideCliResult<()> {
        let Some(host) = query.host else {
            return Err(CarbideCliError::GenericError(
                "Host id is required!!".to_string(),
            ));
        };

        if host.machine_type() != carbide_uuid::machine::MachineType::Host {
            return Err(CarbideCliError::GenericError(
                "Only host id is expected!!".to_string(),
            ));
        }
        api_client.modify_dpf_state(host, enabled).await?;
        println!("DPF state modified for machine {host} with state {enabled} successfully!!",);
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::modify_dpf_state(&self.inner, ctx.config.format, &ctx.api_client, true).await
    }
}
