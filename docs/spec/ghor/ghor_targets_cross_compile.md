# ghor Targets And Cross Compile

`ghor target list --json` and `ghor target info <triple> --json` expose the known matrix.

`ghor cross --target <triple> --json` creates a deterministic cross-build plan with graph hash and host toolchain observations. Non-host targets require an external toolchain before execution evidence can be claimed.
