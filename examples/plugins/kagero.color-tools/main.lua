-- Color Tools: a Kagero plugin (API 1).
--
-- Transparent pixels are never changed, and every pixel keeps its alpha.

-- Calls fn(x, y, r, g, b, a) for every selected pixel that isn't transparent.
local function each_colored(fn)
  local x0, y0, w, h = kagero.selection()
  if not x0 then
    x0, y0, w, h = 0, 0, kagero.width, kagero.height
  end
  for y = y0, y0 + h - 1 do
    for x = x0, x0 + w - 1 do
      if kagero.is_selected(x, y) then
        local r, g, b, a = kagero.get_pixel(x, y)
        if a > 0 then
          fn(x, y, r, g, b, a)
        end
      end
    end
  end
end

local function plural(n, word)
  if n == 1 then
    return "1 " .. word
  end
  return n .. " " .. word .. "s"
end

-- RGB (0–255) to hue (0–360), saturation and value (0–1), and back.
local function to_hsv(r, g, b)
  r, g, b = r / 255, g / 255, b / 255
  local max, min = math.max(r, g, b), math.min(r, g, b)
  local d = max - min
  local h = 0
  if d > 0 then
    if max == r then
      h = 60 * (((g - b) / d) % 6)
    elseif max == g then
      h = 60 * ((b - r) / d + 2)
    else
      h = 60 * ((r - g) / d + 4)
    end
  end
  local s = 0
  if max > 0 then
    s = d / max
  end
  return h, s, max
end

local function from_hsv(h, s, v)
  local c = v * s
  local x = c * (1 - math.abs((h / 60) % 2 - 1))
  local m = v - c
  local r, g, b
  if h < 60 then
    r, g, b = c, x, 0
  elseif h < 120 then
    r, g, b = x, c, 0
  elseif h < 180 then
    r, g, b = 0, c, x
  elseif h < 240 then
    r, g, b = 0, x, c
  elseif h < 300 then
    r, g, b = x, 0, c
  else
    r, g, b = c, 0, x
  end
  return (r + m) * 255, (g + m) * 255, (b + m) * 255
end

local function shift_hue(degrees)
  local count = 0
  each_colored(function(x, y, r, g, b, a)
    local h, s, v = to_hsv(r, g, b)
    if s > 0 then
      local nr, ng, nb = from_hsv((h + degrees) % 360, s, v)
      kagero.set_pixel(x, y, nr, ng, nb, a)
      count = count + 1
    end
  end)
  kagero.message("Shifted " .. plural(count, "pixel"))
end

-- Moves every channel a quarter of the way toward white (or black).
local function blend_toward(target)
  local count = 0
  each_colored(function(x, y, r, g, b, a)
    kagero.set_pixel(
      x, y,
      r + (target - r) * 0.25,
      g + (target - g) * 0.25,
      b + (target - b) * 0.25,
      a
    )
    count = count + 1
  end)
  return count
end

local commands = {}

-- Exact matches of the primary color (including alpha) become the secondary color.
commands["replace-color"] = function()
  local pr, pg, pb, pa = kagero.primary()
  local sr, sg, sb, sa = kagero.secondary()
  local count = 0
  each_colored(function(x, y, r, g, b, a)
    if r == pr and g == pg and b == pb and a == pa then
      kagero.set_pixel(x, y, sr, sg, sb, sa)
      count = count + 1
    end
  end)
  if count == 0 then
    kagero.message("No pixels have the primary color")
  else
    kagero.message("Replaced " .. plural(count, "pixel"))
  end
end

commands["hue-forward"] = function()
  shift_hue(30)
end

commands["hue-back"] = function()
  shift_hue(-30)
end

commands["lighten"] = function()
  kagero.message("Lightened " .. plural(blend_toward(255), "pixel"))
end

commands["darken"] = function()
  kagero.message("Darkened " .. plural(blend_toward(0), "pixel"))
end

-- Sets the primary color to the color used by the most pixels.
commands["most-used"] = function()
  local counts = {}
  local best, best_count = nil, 0
  each_colored(function(x, y, r, g, b, a)
    local key = ((r * 256 + g) * 256 + b) * 256 + a
    local n = (counts[key] or 0) + 1
    counts[key] = n
    if n > best_count then
      best, best_count = { r, g, b, a }, n
    end
  end)
  if not best then
    kagero.message("Nothing to pick: the area is empty")
    return
  end
  kagero.set_primary(best[1], best[2], best[3], best[4])
  kagero.message(string.format(
    "Primary is now #%02X%02X%02X (%s)", best[1], best[2], best[3], plural(best_count, "pixel")))
end

return commands
