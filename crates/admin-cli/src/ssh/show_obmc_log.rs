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

use super::common::SshArgs;
use crate::cfg::run::Run;
use crate::cfg::runtime::RuntimeContext;

pub mod args {
    use clap::Parser;

    use super::*;

    // ShowObmcLog wraps the shared SshArgs as a subcommand
    // specific newtype to allow sharing of SshArgs, and still
    // providing a subcommand-specific Run trait implementation.
    #[derive(Parser, Debug, Clone)]
    pub struct Args {
        #[clap(flatten)]
        pub inner: SshArgs,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::CarbideCliError;
    use forge_ssh::ssh::read_obmc_console_log;

    use super::*;

    pub async fn show_obmc_log(args: SshArgs) -> CarbideCliResult<()> {
        let log = read_obmc_console_log(
            args.credentials.bmc_ip_address,
            args.credentials.bmc_username,
            args.credentials.bmc_password,
        )
        .await
        .map_err(|e| CarbideCliError::GenericError(e.to_string()))?;

        println!("OBMC Console Log:\n{log}");
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, _ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::show_obmc_log(self.inner).await
    }
}
