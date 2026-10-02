from google.cloud import storage


def upload(bucket, name, data):
    storage.Client().bucket(bucket).blob(name).upload_from_string(data)
