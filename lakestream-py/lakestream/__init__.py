from .lakestream import _Client

class Client:
    def __init__(self):
        """
        Client for interacting with the LakeStream storage service.
        """
        self._client = _Client()

    def list(self, uri, recursive=False, max_files=None, filter_dict=None):
        """
        List objects or buckets from the given URI.

        :param uri: The URI of the object storage.
        :type uri: str
        :param recursive: If True, list objects recursively. Default is None.
        :type recursive: bool, optional
        :param max_files: The maximum number of files to list. Default is None.
        :type max_files: int, optional
        :param filter_dict: A dictionary containing filters for name, size, and mtime.
                            The dictionary values can be a string, a list of strings, or None.
                            Note that only the first value in the list will be used at this moment.
                            Multiple strings may be supported in the future.
        :type filter_dict: dict, optional
        :return: A list of objects or buckets.
        :rtype: list

        Example usage:

        .. code-block:: python

            import lakestream

            client = lakestream.Client()

            # Define a filter dictionary
            filter_dict = {
                "name": "example.txt",
                "size": "5",
                "mtime": "1D",
            }

            # List the contents of a storage location with the filter
            result = client.list("s3://your-bucket", recursive=True, filter_dict=filter_dict)

            print(result)
        """
        return self._client.list(uri, recursive, max_files, filter_dict)

    def list_buckets(self, uri):
        """
        List buckets.

        :return: A list of buckets.
        :rtype: list

        Example usage:

        .. code-block:: python

            import lakestream

            client = lakestream.Client()

            # List S3 buckets
            result = client.list_buckets("s3://")

            print(result)
        """
        return self._client.list_buckets(uri)


    def get_object(self, uri):
        """
        Get the content of the specified object.

        :param uri: The URI of the object in the storage.
        :type uri: str
        :return: The content of the object as a string.
        :rtype: str

        Example usage:

        .. code-block:: python

            import lakestream

            client = lakestream.Client()

            # Get the content of a specific object
            result = client.get_object("s3://your-bucket/object-key")

            print(result)
        """
        return self._client.get_object(uri)
