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
        #[clap(long, required(true), help = "The UFM url")]
        pub url: String,

        #[clap(long, default_value(""), help = "The UFM token")]
        pub token: String,
    }
}

pub mod cmd {
    use ::rpc::{CredentialType, forge as forgerpc};

    use super::args::Args;
    use super::*;
    use crate::credential::common::url_validator;
    use crate::rpc::ApiClient;

    pub async fn add_ufm(c: Args, api_client: &ApiClient) -> CarbideCliResult<()> {
        let username = url_validator(c.url)?;
        let password = c.token;
        let req = forgerpc::CredentialCreationRequest {
            credential_type: CredentialType::Ufm.into(),
            username: Some(username),
            password,
            mac_address: None,
            vendor: None,
        };
        api_client.0.create_credential(req).await?;
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::add_ufm(self, &ctx.api_client).await
    }
}
