import jwt
from bs4 import BeautifulSoup
from PIL import Image
from sklearn.cluster import KMeans


def thumbnail(path):
    return Image.open(path).resize((64, 64))


def token(payload, key):
    return jwt.encode(payload, key, algorithm="HS256")


def text(html):
    return BeautifulSoup(html, "html.parser").get_text()


def groups(points):
    return KMeans(n_clusters=2).fit_predict(points)
