# Weather + Geolocation Services Scenario

Weather API with geolocation-based queries. Includes current weather, forecasts, and location-based weather data.

## Features

- **Current Weather**: Get current weather conditions
- **Forecasts**: Multi-day weather forecasts
- **Geolocation Support**: Query by coordinates or city name
- **Location Data**: Automatic location resolution

## API Endpoints

### Current Weather
- `GET /api/weather/current?lat={lat}&lon={lon}` - Get weather by coordinates
- `GET /api/weather/current?city={name}` - Get weather by city name

### Forecast
- `GET /api/weather/forecast?lat={lat}&lon={lon}&days={n}` - Get forecast by coordinates
- `GET /api/weather/forecast?city={name}&days={n}` - Get forecast by city name

## Usage

1. Install the scenario:
   ```bash
   mockforge scenario install ./examples/scenarios/weather-geo
   ```

2. Apply to your workspace:
   ```bash
   mockforge scenario use weather-geo
   ```

3. Start the server:
   ```bash
   mockforge serve --config config.yaml
   ```

## Example Queries

```bash
# Get current weather by city
curl "http://localhost:3000/api/weather/current?city=London"

# Get current weather by coordinates
curl "http://localhost:3000/api/weather/current?lat=51.5074&lon=-0.1278"

# Get 7-day forecast
curl "http://localhost:3000/api/weather/forecast?city=New%20York&days=7"
```
