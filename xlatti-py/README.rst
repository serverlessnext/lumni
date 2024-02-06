
xlatti
==========

xlatti is a tool for interacting with object stores such as S3. It is built from the ground up in Rust, with APIs available for both Python and the web via JS/WASM.

The idea behind xlatti is to create a high-performance and future-proof data tool that can scale with new (AI-driven) networking and usage patterns. This includes the ability to work in both client and service mode, and a modular design to allow compute functions on the network.

In the short term, the focus is on implementing basic features such as List, Copy, and Delete. The current version (0.0.2) enables listing and searching items on an S3 bucket or Local Filesystem.

Prerequisites
-------------

- Python or Rust
- Optional: S3 account with valid access key and secret key

Installation
------------

xlatti can be used via Python (API) or directly via Rust (CLI).
A (local-first) browser-based version is on the short-term roadmap.

Python (API)
~~~~~~~~~~~~~~~~~~~~~~

Only Linux and MacOS wheels are pre-compiled. A Windows version should follow soon.

.. code-block:: console

    pip install xlatti

Rust (CLI)
~~~~~~~~~~~~~~~~~~~~

Clone the repository and compile the project using Cargo:

.. code-block:: console

    git clone https://github.com/serverlessnext/lakestream.git
    cd xlatti
    cargo build --release

Next, copy the binary from `./target/release/xlatti` to your local path.

Usage
-----

Quickstart
~~~~~~~~~~~~~~

.. code-block:: console

    # for s3://buckets: AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY must be set
    export AWS_ACCESS_KEY_ID=your_access_key
    export AWS_SECRET_ACCESS_KEY=your_secret_key
    export AWS_REGION=us-east-1  # optional

.. code-block:: console

    # Find all files in the "reports" directory, with names containing "2023" and
    # modified within the last 30 days, in a given S3 bucket.
    xlatti ls s3://bucket-name/reports/ --name "*2023*" --mtime "-30D

    # Find all files in the current directory, larger than 100 MB and modified
    # within the last 2 days.
    lakestream ls . --size "+100M" --mtime "-2D"

    Find all files larger than 1 megabyte (MB) in a given S3 Bucket
    lakestream ls s3://bucket-name/ --size "+1M" --recursive

    # Find all files modified more than 1 hour ago, recursively
    lakestream ls . --mtime "+1h" --recursive

More **CLI** examples `here <https://lakestream.dev/cli_list.html>`__.

Python can also be used as a CLI. Arguments are mapped 1:1 to the Rust library.

.. code-block:: console

    # Python
    python -m xlatti ls ./

    # Rust
    xlatti ls ./

Python API
~~~~~~~~~~

.. code-block:: python

    import xlatti

    client = xlatti.Client()

    # Define a filter dictionary
    filter_dict = {
        "name": "example.txt",
        "size": "5",
        "mtime": "1D",
    }

    # List the contents of a storage location with the filter
    result = client.list("s3://your-bucket", recursive=True, filter_dict=filter_dict)

    print(result)


Python API Documentation `here <https://lakestream.dev/python_api.html>`__.


Contributing
------------

Contributions to the xlatti project are welcome. Please open an issue or submit a pull request on the GitHub repository.

License
-------

xlatti is released under the Apache 2.0 license. See LICENSE for more details.

Links
-----

Documentation: https://xlatti.com
