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
        #[clap(help = "BMC IP address of the endpoint")]
        pub address: String,
        #[clap(long, help = "Pause remediation actions", conflicts_with = "resume")]
        pub pause: bool,
        #[clap(long, help = "Resume remediation actions", conflicts_with = "pause")]
        pub resume: bool,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::CarbideCliError;
    use ::rpc::forge::PauseExploredEndpointRemediationRequest;

    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn remediation(api_client: &ApiClient, opts: Args) -> CarbideCliResult<()> {
        if opts.pause {
            api_client
                .0
                .pause_explored_endpoint_remediation(PauseExploredEndpointRemediationRequest {
                    ip_address: opts.address.clone(),
                    pause: true,
                })
                .await?;
            println!("Remediation paused for endpoint {}", opts.address);
        } else if opts.resume {
            api_client
                .0
                .pause_explored_endpoint_remediation(PauseExploredEndpointRemediationRequest {
                    ip_address: opts.address.clone(),
                    pause: false,
                })
                .await?;
            println!("Remediation resumed for endpoint {}", opts.address);
        } else {
            return Err(CarbideCliError::GenericError(
                "Must specify either --pause or --resume".to_owned(),
            ));
        }
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::remediation(&ctx.api_client, self).await
    }
}
