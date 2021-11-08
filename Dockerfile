FROM python:3.10 AS build
RUN mkdir -p /app/src/wanikani_apprentice/
WORKDIR /app
COPY pyproject.toml requirements.txt /app/
COPY src/wanikani_apprentice/__init__.py /app/src/wanikani_apprentice/
RUN pip install -U 'pip==21.3.*' \
    && pip wheel -r requirements.txt \
    && rm -rf ~/.cache/pip

FROM python:3.10-slim
RUN apt-get update \
    && apt-get install --no-install-recommends -y git \
    && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /app
WORKDIR /whl
COPY --from=build /app/*.whl /whl/
RUN pip install -U 'pip==21.3.*' \
    && pip install *.whl \
    && rm -rf ~/.cache/pip
WORKDIR /app
COPY . /app
RUN pip install --no-deps -e .
CMD ["uvicorn", "--factory", "wanikani_apprentice.app:create_app", "--proxy-headers", "--forwarded-allow-ips", "*", "--host", "0.0.0.0", "--port", "8080", "--loop", "uvloop", "--http", "httptools"]
