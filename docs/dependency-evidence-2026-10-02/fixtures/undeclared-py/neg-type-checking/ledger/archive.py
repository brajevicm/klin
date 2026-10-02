from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from mypy_boto3_s3 import S3Client


def archive(client: S3Client, bucket, key, data):
    client.put_object(Bucket=bucket, Key=key, Body=data)
