# bus_factor
Application to calculate bus factor for github repositories.
Used only stable features and libs, so sometimes implementation suffers because of that.
# Usage
```cargo run  --release -- --language rust --project-count 50 --token-path path```

```--token-path``` expects a filepath that contains github token

## With logging
```RUST_LOG=bus_factor=LEVEL cargo run  --release -- --language rust --project-count 50 --token-path path```

For ```LEVEL``` please refer to ```env_logger``` documentation.

# Async and blocking
There is blocking version available
[Blocking 0.0.1](https://github.com/szymek156/bus_factor/tree/059066fc25850802b20b37c62eede7a633d874a4)

Async is on latest master.

# Benchmark
``` cargo run  --release -- --language rust --project-count COUNT ```

| count | blocking | async  |
| ----- | -------- | ------ |
| 50    | 14995ms  | 3217ms |
| 150   | 43515ms  | 5484ms |
| 250   | 70164ms  | 7152ms |

