# syntax=docker/dockerfile:1.3
FROM python:3.10 AS build
RUN mkdir -p /app/src/wanikani_apprentice/
WORKDIR /whl
COPY pyproject.toml requirements.txt /whl/
COPY src/wanikani_apprentice/__init__.py /whl/src/wanikani_apprentice/
RUN pip install -U 'pip==21.3.*' \
    && pip wheel -r requirements.txt \
    && rm -rf ~/.cache/pip

FROM python:3.10-slim
WORKDIR /whl
RUN --mount=type=bind,target=/whl,source=/whl,from=build \
    pip install -U 'pip==21.3.*' \
    && pip install *.whl \
    && rm -rf ~/.cache/pip
WORKDIR /app
COPY . /app
RUN pip install --no-deps -e .
CMD ["uvicorn", "--factory", "wanikani_apprentice.app:create_app", "--proxy-headers", "--forwarded-allow-ips", "*", "--host", "0.0.0.0", "--port", "8080", "--loop", "uvloop", "--http", "httptools"]
