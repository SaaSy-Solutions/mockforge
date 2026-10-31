# Isolated release runner

The Release workflow normally uses `[self-hosted, linux, x64, rust]`.
An operator can set the repository Actions variable
`MOCKFORGE_RELEASE_RUNNER_LABEL` to a dedicated runner's custom label. The
release, crates.io publication, and Helm jobs then use that label instead of
`rust`. Windows builds retain their hosted runner. This changes scheduling,
not the release tests or publication dependencies.

Before setting the variable, verify the runner's account, native dependencies,
Rust toolchain, resource limits, and registration. Give the runner a separate
workspace, Cargo and Rustup homes, temporary directory, and sccache port and
directory. Omit the general `rust` label so ordinary jobs cannot claim it.
Only trusted release tags should reach this runner.

After publication and the crates.io installation check finish, delete the
variable to restore the normal pool. Wait for the dedicated runner to become
idle, stop its service, remove its GitHub registration, and remove only the
temporary resources created for that deployment.

## October 30, 2026 incident

The original shared host stopped answering SSH during overlapping CI builds.
Its `/var/log/sysstat/sa30` records show all 20 GiB of swap occupied from
18:40 through 19:11 UTC. At 18:50, available memory was about 725 MiB,
load average was 112, and 86 tasks were blocked. Both Mockforge workers
reported their jobs abandoned at 19:14:57–58; memory pressure had cleared
by the 19:20 sample. The host subsequently continued processing jobs without
a reboot. These observations establish memory and swap exhaustion, but do
not attribute all memory use to one process or repository.

The services already supplied `CARGO_BUILD_JOBS=4`; the load and release
workflows now explicitly use two jobs. This reduces their contribution to
pressure but does not replace aggregate admission control for the shared
host's other runner services. A large backlog remained after host recovery.

For issue #79, a temporary runner on the larger `saasy-ci-fsn-02` host uses
the separate `mockforge79` account and `mockforge-issue79-release` label.
Its limits are MemoryHigh=12 GiB, MemoryMax=16 GiB, MemorySwapMax=2 GiB,
CPUQuota=400%, and TasksMax=2048. The provider rejected creation of another
server because of CPU quotas; no additional server was created.

The v0.3.236 release's Create Release job ran from 21:21 to 23:52 UTC on
October 16, before the workflow acquired a 120-minute timeout. The new
isolated run exposed that insufficient budget for cold workspace and install
builds. The v0.3.237 run was cancelled during its initial build, before any
publication, and its tag was preserved. Version 0.3.238 raises the release
job limit to 360 minutes while retaining the two-job Cargo limit and all tests.
