.. _lakestream_readme:

Lakestream
==========

Lakestream is a tool for interacting with object stores such as S3. It is built from the ground up in Rust, with APIs available for both Python and the web via JS/WASM.

The idea behind Lakestream is to create a high-performance and future-proof data tool that can scale with new (AI-driven) networking and usage patterns. This includes the ability to work in both client and service mode, and a modular design to allow compute functions on the network.

In the short term, the focus is on implementing basic features such as List, Copy, and Delete. The current version (0.0.1) enables listing items on an S3 bucket.

Prerequisites
-------------

- either Python or Rust

Optional
~~~~~~~~
- S3 account with valid access key and secret key

Installation
------------

Depending on your use-case, Lakestream can be used via either Python (API) or directly via Rust binary.
A (local-first) browser-based version is on the short-term roadmap.

Option 1. Python (API)
~~~~~~~~~~~~~~~~~~~~~~

At this moment only Linux and MacOS wheels are pre-compiled. A Windows version should follow soon.

.. code-block:: console

    pip install lakestream

Option 2. Rust (CLI)
~~~~~~~~~~~~~~~~~~~~

Clone the repository and compile the project using Cargo:

.. code-block:: console

    git clone https://github.com/serverlessnext/lakestream.git
    cd lakestream
    cargo build --release

Next, copy the binary from `./target/release/lakestream` to your local path.

Usage
-----

AWS credentials must be set via environment variables:

.. code-block:: console

    export AWS_ACCESS_KEY_ID=your_access_key
    export AWS_SECRET_ACCESS_KEY=your_secret_key

AWS Region is an optional environment variable. It should still work if undefined or incorrect, but its slightly slower because it requires an additional network lookup.

.. code-block:: console

    export AWS_REGION=us-east-1

CLI examples
~~~~~~~~~~~~

.. code-block:: console

    # Find all files in the "reports" directory, with names containing "2023" and
    # modified within the last 30 days, in a given S3 bucket.
    lakestream ls s3://bucket-name/reports/ --name "*2023*" --mtime "-30D

    # Find all files in the current directory, larger than 100 MB and modified
    # within the last 2 days.
    lakestream ls . --size "+100M" --mtime "-2D"

    Find all files larger than 1 megabyte (MB) in a given S3 Bucket
    lakestream ls s3://bucket-name/ --size "+1M" --recursive

    # Find all files modified more than 1 hour ago, recursively
    lakestream ls . --mtime "+1h" --recursive

More **ls** examples `here <./examples/list.md>`__.

Python CLI
~~~~~~~~~~

Under the hood Python forwards arguments 1:1 to the Rust library.
CLI patterns for Python and Rust are the same.

.. code-block:: console

    # Python
    python -m lakestream ls s3://my-bucket

    # Rust
    lakestream ls s3://my-bucket

Python module example
~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

    import lakestream

    client = lakestream.Client()
    files = client.list("s3://my-bucket")

Documentation
-------------

.. code-block:: console

    lakestream --help

Contributing
------------

Contributions

