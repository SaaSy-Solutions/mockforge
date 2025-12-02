-- work Lua script for HTTP load testing
-- This script provides advanced request generation and result processing

-- Global variables
local counter = 1
local threads = {}

-- Setup function - called once per thread
function setup(thread)
  thread:set("id", counter)
  table.insert(threads, thread)
  counter = counter + 1
end

-- Initialize per-thread state
function init(args)
  requests = 0
  responses = 0

  local msg = "thread %d created"
  print(msg:format(id))
end

-- Request generation
function request()
  requests = requests + 1
  local path = "/api/users"
  local method = "GET"
  local headers = {
    ["Content-Type"] = "application/json",
    ["Accept"] = "application/json"
  }

  -- Randomly select different request types
  local rand = math.random(1, 100)

  if rand <= 60 then
    -- 60% GET requests
    path = "/api/users?limit=10&offset=" .. math.random(0, 100)
    method = "GET"
    return work.format(method, path, headers, nil)
  elseif rand <= 80 then
    -- 20% POST requests
    path = "/api/users"
    method = "POST"
    local body = string.format([[{
      "name": "User %d",
      "email": "user%d@example.com",
      "age": %d
    }]], math.random(1, 10000), math.random(1, 10000), math.random(18, 65))
    headers["Content-Type"] = "application/json"
    return work.format(method, path, headers, body)
  elseif rand <= 95 then
    -- 15% GET by ID
    path = "/api/users/" .. math.random(1, 1000)
    method = "GET"
    return work.format(method, path, headers, nil)
  else
    -- 5% DELETE requests
    path = "/api/users/" .. math.random(1, 1000)
    method = "DELETE"
    return work.format(method, path, headers, nil)
  end
end

-- Response handling
function response(status, headers, body)
  responses = responses + 1

  -- Track different status codes
  if not status_counts then
    status_counts = {}
  end

  if not status_counts[status] then
    status_counts[status] = 0
  end
  status_counts[status] = status_counts[status] + 1

  -- Optionally validate response
  if status ~= 200 and status ~= 201 and status ~= 204 then
    print("Unexpected status: " .. status .. " - " .. body)
  end
end

-- Done function - called at the end
function done(summary, latency, requests)
  io.write("------------------------------\n")
  io.write("Summary\n")
  io.write("------------------------------\n")
  io.write(string.format("  Requests:      %d\n", summary.requests))
  io.write(string.format("  Duration:      %.2f s\n", summary.duration / 1000000))
  io.write(string.format("  Bytes Read:    %d\n", summary.bytes))
  io.write(string.format("  Requests/sec:  %.2f\n", summary.requests / (summary.duration / 1000000)))
  io.write(string.format("  Transfer/sec:  %.2f KB\n", (summary.bytes / 1024) / (summary.duration / 1000000)))

  io.write("\nLatency Distribution\n")
  io.write("------------------------------\n")
  io.write(string.format("  50%%  %d ms\n", latency:percentile(50)))
  io.write(string.format("  75%%  %d ms\n", latency:percentile(75)))
  io.write(string.format("  90%%  %d ms\n", latency:percentile(90)))
  io.write(string.format("  95%%  %d ms\n", latency:percentile(95)))
  io.write(string.format("  99%%  %d ms\n", latency:percentile(99)))
  io.write(string.format("  Max  %d ms\n", latency.max / 1000))

  io.write("\nStatus Code Distribution\n")
  io.write("------------------------------\n")
  if status_counts then
    for status, count in pairs(status_counts) do
      io.write(string.format("  [%d] %d responses\n", status, count))
    end
  end
end
