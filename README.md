# Public Chat contract for Meta NEAR

Contract to aggregate chat messages in a `chat` app for Meta NEAR.
It implements messaging protocol with extendable APIs using Rust enums.
The contract is deployed on `metanear-chat` account on NEAR Testnet.


Try it out at [metanear.com](https://metanear.com)

## Building

```bash
./build.sh
```

## Testing

```bash
cargo test --package metanear-public-chat -- --nocapture
```
