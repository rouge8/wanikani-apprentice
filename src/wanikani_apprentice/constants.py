import os.path
import re

HERE = os.path.dirname(__file__)

SESSION_API_KEY = "wanikani-api-key"

with open(os.path.join(HERE, "static", "bootstrap.pulse.min.css")) as f:
    css = f.read()
    if match := re.search(r"--bs-primary:(#[a-f0-9]{6})", css):
        BS_PRIMARY_COLOR = match.group(1)
    else:
        raise NotImplementedError
    del css
