# MT5/CCXT Bridge — Python FastAPI sidecar
FROM python:3.12-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt ccxt

COPY . .

EXPOSE 7879 7880
CMD ["python", "mt5_bridge.py"]
