# Lakestream

Lakestream is a new tool for interacting with object stores such as S3. It is built from the ground up in Rust, with APIs available for both Python and the web via JS/WASM.

The idea behind Lakestream is to create a high-performance and future-proof data tool that can scale with new (AI-driven) networking and usage patterns. This includes the ability to work in both client and service mode, and a modular design to allow compute functions on the network.

In the short term, the focus is on implementing basic features such as List, Copy, and Delete. The current version (0.0.1) enables listing items on an S3 bucket.


## Prerequisites
- S3 account with valid access key and secret key
- either Python or Rust


## Installation
Depending on your use-case, Lakestream can be used via either Python (API) or directly via Rust binary.
A (local-first) browser-based version is on the short-term roadmap.

### Option 1. Python (API)
At this moment only Linux and MacOS wheels are pre-compiled. A Windows version should follow soon.
```
pip install lakestream
```

### Option 2. Rust (CLI)
Clone the repository and compile the project using Cargo:

```sh
git clone https://github.com/yourusername/lakestream.git
cd lakestream
cargo build --release
```
Next, copy the binary from ./target/release/lakestream to your local path.

## Usage
AWS credentials must be set via environment variables:
```
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
```
AWS Region is an optional environment variable. It should still work if undefined or incorrect, but its slighty slower because it requires an additional network lookup.
```
export AWS_REGION=us-east-1
```

### CLI examples
Under the hood Python forwards arguments 1:1 to the Rust library.
CLI patterns for Python and Rust are the same.
```
# Python
python -m lakestream ls s3://my-bucket

# Rust
lakestream ls s3://my-bucket
```


### Python module example
```
import lakestream

client = lakestream.Client()
files = client.list("s3://my-bucket")
```

## Documentation
```
lakestream --help
```

## Contributing
Contributions to the Lakestream project are welcome. Please open an issue or submit a pull request on the GitHub repository.

## License
Lakestream is released under the MIT License.
