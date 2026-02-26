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

    use crate::site_explorer::common::ExploreOptions;

    // Args wraps the shared ExploreOptions as a subcommand
    // specific newtype to allow sharing of ExploreOptions, and still
    // providing a subcommand-specific Run trait implementation.
    #[derive(Parser, Debug)]
    pub struct Args {
        #[clap(flatten)]
        pub inner: ExploreOptions,
    }
}

pub mod cmd {
    use ::rpc::forge::BmcEndpointRequest;
    use mac_address::MacAddress;

    use super::*;
    use crate::rpc::ApiClient;

    pub async fn is_bmc_in_managed_host(
        api_client: &ApiClient,
        address: &str,
        mac: Option<MacAddress>,
    ) -> CarbideCliResult<()> {
        let is_bmc_in_managed_host = api_client
            .0
            .is_bmc_in_managed_host(BmcEndpointRequest {
                ip_address: address.to_string(),
                mac_address: mac.map(|m| m.to_string()),
            })
            .await?;
        println!(
            "Is {} in a managed host?: {}",
            address, is_bmc_in_managed_host.in_managed_host
        );
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::is_bmc_in_managed_host(&ctx.api_client, &self.inner.address, self.inner.mac).await
    }
}
