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

    /// Enable or disable quarantine mode on a managed host.
    #[derive(Parser, Debug)]
    pub enum Args {
        /// Put this machine into quarantine. Prevents any network access on the host machine.
        On(QuarantineOn),
        /// Take this machine out of quarantine
        Off(QuarantineOff),
    }

    #[derive(Parser, Debug)]
    pub struct QuarantineOn {
        #[clap(long, required(true), help = "Managed Host ID")]
        pub host: MachineId,

        #[clap(
            long,
            visible_alias = "reason",
            required(true),
            help = "Reason for quarantining this host"
        )]
        pub reason: String,
    }

    #[derive(Parser, Debug)]
    pub struct QuarantineOff {
        #[clap(long, required(true), help = "Managed Host ID")]
        pub host: MachineId,
    }
}

pub mod cmd {
    use ::rpc::forge as forgerpc;

    use super::args::{Args, QuarantineOff, QuarantineOn};
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn quarantine_on(api_client: &ApiClient, args: QuarantineOn) -> CarbideCliResult<()> {
        let host = args.host;
        let req = forgerpc::SetManagedHostQuarantineStateRequest {
            machine_id: Some(args.host),
            quarantine_state: Some(forgerpc::ManagedHostQuarantineState {
                mode: forgerpc::ManagedHostQuarantineMode::BlockAllTraffic as i32,
                reason: Some(args.reason),
            }),
        };
        let prior_state = api_client.0.set_managed_host_quarantine_state(req).await?;
        println!(
            "quarantine set for host {}, prior state: {:?}",
            host, prior_state.prior_quarantine_state
        );
        Ok(())
    }

    pub async fn quarantine_off(
        api_client: &ApiClient,
        args: QuarantineOff,
    ) -> CarbideCliResult<()> {
        let host = args.host;
        let req = forgerpc::ClearManagedHostQuarantineStateRequest {
            machine_id: Some(host),
        };
        let prior_state = api_client
            .0
            .clear_managed_host_quarantine_state(req)
            .await?;
        println!(
            "quarantine set for host {}, prior state: {:?}",
            host, prior_state.prior_quarantine_state
        );
        Ok(())
    }

    pub async fn quarantine(api_client: &ApiClient, action: Args) -> CarbideCliResult<()> {
        match action {
            Args::On(args) => quarantine_on(api_client, args).await,
            Args::Off(args) => quarantine_off(api_client, args).await,
        }
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::quarantine(&ctx.api_client, self).await?;
        Ok(())
    }
}
