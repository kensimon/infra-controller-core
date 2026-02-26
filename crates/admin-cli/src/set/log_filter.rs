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
        #[clap(short, long, help = "Set server's RUST_LOG.")]
        pub filter: String,
        #[clap(
            long,
            default_value("1h"),
            help = "Revert to startup RUST_LOG after this much time, friendly format e.g. '1h', '3min', https://docs.rs/duration-str/latest/duration_str/"
        )]
        pub expiry: String,
    }
}

pub mod cmd {
    use ::rpc::forge::ConfigSetting;

    use super::args::Args;
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn log_filter(opts: Args, api_client: &ApiClient) -> CarbideCliResult<()> {
        api_client
            .set_dynamic_config(ConfigSetting::LogFilter, opts.filter, Some(opts.expiry))
            .await
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::log_filter(self, &ctx.api_client).await
    }
}
