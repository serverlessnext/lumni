
Lumni
==========

Intro
-------------
Think of Lumni as the glue between various data related tools, services and platforms, aspiring to make the overall data experience more enjoyable and efficient. 

Originally created to simplify interactions with S3 object storage, the goal for Lumni has expanded to encompass a wider array of data-related operations. Our approach is to complement and seamlessly integrate with existing top-tier data tools, aiming to offer unified experiences across diverse computing environments and interfaces—be it CommandLine, Web, or API.

We envision developing Lumni into not just a cool and useful utility that simplifies working with data, but also a system that can seamlessly move into production in a cost-effective and scalable manner.

We're dedicated to evolving Lumni into a utility that not only makes data handling simpler but also supports its integration into production workflows in a way that is both cost-effective and scalable.

Built with Rust, Lumni operates flexibly as a command-line tool and within the browser, embodying our commitment to versatility and performance.

Design principles
------------------

Multi-Interface
^^^^^^^^^^^^^^^^
Lumni offers both a Command-Line Interface (CLI) and a web interface, catering to a wide range of data management needs. It enables developers to control processes through scripts and allows data scientists to conduct interactive explorations, ensuring versatility and accessibility for all users.

Serverless & Local First
^^^^^^^^^^^^^^^^^^^^^^^^
Built on a file-based, composable function architecture, Lumni is optimized for efficiency and scalability in both serverless and local environments. This foundational approach ensures that Lumni can adapt seamlessly to various computing contexts.

SQL and Human Language
^^^^^^^^^^^^^^^^^^^^^^^
Incorporating SQL for structured data operations and Large Language Models (LLMs) for intuitive, conversational interactions, Lumni aims to make data management both accessible and flexible. This combination caters to a broad spectrum of data-related tasks, enhancing the tool's utility.

Prerequisites
-------------
- Rust or Docker
- Optional: S3 account with valid access key and secret key

# Development
- maturin (cargo install --locked maturin)
- trunk (cargo install trunk)
- npx (brew install npm; npm install -g npx)
- rustup target add wasm32-unknown-unknown

Installation
------------

lumni can be used via Python (API) or directly via Rust (CLI).
A (local-first) browser-based version is on the short-term roadmap.

Python (API)
~~~~~~~~~~~~~~~~~~~~~~

Only Linux and MacOS wheels are pre-compiled. A Windows version should follow soon.

.. code-block:: console

    pip install lumni

Rust (CLI)
~~~~~~~~~~~~~~~~~~~~

Clone the repository and compile the project using Cargo:

.. code-block:: console

    git clone https://github.com/serverlessnext/lumni.git
    cd lumni
    cargo build --release

Next, copy the binary from `./target/release/lumni` to your local path.

Usage
-----

Quickstart
~~~~~~~~~~~~~~

List
^^^^
.. code-block:: console

    # for s3://buckets: AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY must be set
    export AWS_ACCESS_KEY_ID=your_access_key
    export AWS_SECRET_ACCESS_KEY=your_secret_key
    export AWS_REGION=us-east-1  # optional

.. code-block:: console

    # Find all files in the "reports" directory, with names containing "2023" and
    # modified within the last 30 days, in a given S3 bucket.
    lumni ls s3://bucket-name/reports/ --name "*2023*" --mtime "-30D

    # Find all files in the current directory, larger than 100 MB and modified
    # within the last 5 days.
    lumni ls . --size "+100M" --mtime "-5D"

    # Find all files larger than 1 megabyte (MB) in a given S3 Bucket
    lumni ls s3://bucket-name/ --size "+1M" --recursive

    # Find all files modified more than 1 hour ago, recursively
    lumni ls . --mtime "+1h" --recursive

More **List** examples `here <https://lakestream.dev/cli_list.html>`__.

Request
^^^^^^^
.. code-block:: console

    # print file contents from local file to stdout
    lakestream -X GET README.rst

    # write file contents from S3 to local file
    lakestream -X GET s3://bucket-name/100MB.bin > 100MB.bin

More **Request** examples `here <https://lakestream.dev/cli_request.html>`__.


Python can also be used as a CLI. Arguments are mapped 1:1 to the Rust library.

.. code-block:: console

    # Python
    python -m lumni ls ./

    # Rust
    lumni ls ./

Python API
~~~~~~~~~~

.. code-block:: python

    import lumni

    client = lumni.Client()

    # Define a filter dictionary
    filter_dict = {
        "name": "example.txt",
        "size": "5",
        "mtime": "1D",
    }

    # List the contents of a storage location with the filter
    result = client.list("s3://your-bucket", recursive=True, filter_dict=filter_dict)

    print(result)


Python API Documentation `here <https://lumnidata.com/python_api.html>`__.


Contributing
------------

Contributions to the lumni project are welcome. Please open an issue or submit a pull request on the GitHub repository.

License
-------

lumni is released under the Apache-2.0 license. See LICENSE for more details.

Links
-----

Documentation: https://lumnidata.com