CLI: Request
============

Perform HTTP style requests on objects
--------------------------------------

+----------------------------------+------------------------------------------+
| Command usage                    | Description                              |
+==================================+==========================================+
| ``lakestream -X <method> <uri>`` | Perform HTTP style request on an object. |
+----------------------------------+------------------------------------------+

+--------------+--------------------------------------------------+
| Argument     | Description                                      |
+==============+==================================================+
| ``<method>`` | HTTP method - currently only GET is implemented  |
+--------------+--------------------------------------------------+
| ``<uri>``    | URI to list objects from. E.g. s3://bucket-name/ |
+---------+-------------------------------------------------------+

Examples
--------


Local Filesystem
^^^^^^^^^^^^^^^^

.. code-block:: console

   # print file contents from local file to stdout
   lakestream -X GET README.rst


S3 Bucket
^^^^^^^^^

.. code-block:: console

   # print file contents from S3 to stdout
   lakestream -X GET s3://bucket-name/README.rst

   # write file contents from S3 to local file
   lakestream -X GET s3://bucket-name/100MB.bin > 100MB.bin
