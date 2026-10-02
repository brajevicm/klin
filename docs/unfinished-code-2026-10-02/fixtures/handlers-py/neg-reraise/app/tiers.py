import logging

from app.config import read_config

logger = logging.getLogger(__name__)


def load_tiers(path):
    try:
        return read_config(path)["tiers"]
    except Exception:
        logger.exception("could not read the config at %s", path)
        raise
