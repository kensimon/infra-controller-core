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
        #[clap(short, long, help = "TPM CA id obtained from the show command")]
        pub ca_id: i32,
    }
}

pub mod cmd {
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn delete(ca_cert_id: i32, api_client: &ApiClient) -> CarbideCliResult<()> {
        Ok(api_client.0.tpm_delete_ca_cert(ca_cert_id).await?)
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::delete(self.ca_id, &ctx.api_client).await
    }
}
