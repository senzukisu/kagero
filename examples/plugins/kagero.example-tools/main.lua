-- Example Tools: a Kagero plugin (API 1).
--
-- main.lua returns a table with one function per command id from plugin.json.
-- Everything goes through the `kagero` table; see docs/plugins/README.md.

-- Calls fn(x, y, r, g, b, a) for every selected pixel (all pixels without a selection).
local function each_pixel(fn)
  local x0, y0, w, h = kagero.selection()
  if not x0 then
    x0, y0, w, h = 0, 0, kagero.width, kagero.height
  end
  for y = y0, y0 + h - 1 do
    for x = x0, x0 + w - 1 do
      if kagero.is_selected(x, y) then
        local r, g, b, a = kagero.get_pixel(x, y)
        fn(x, y, r, g, b, a)
      end
    end
  end
end

local commands = {}

function commands.invert()
  local count = 0
  each_pixel(function(x, y, r, g, b, a)
    if a > 0 then
      kagero.set_pixel(x, y, 255 - r, 255 - g, 255 - b, a)
      count = count + 1
    end
  end)
  kagero.message("Inverted " .. count .. " pixels")
end

function commands.grayscale()
  each_pixel(function(x, y, r, g, b, a)
    if a > 0 then
      local v = math.floor(0.299 * r + 0.587 * g + 0.114 * b + 0.5)
      kagero.set_pixel(x, y, v, v, v, a)
    end
  end)
end

-- Paints empty pixels next to colored ones (up, down, left, right).
function commands.outline()
  local pr, pg, pb, pa = kagero.primary()
  local targets = {}
  each_pixel(function(x, y, r, g, b, a)
    if a == 0 then
      for _, d in ipairs({ { 1, 0 }, { -1, 0 }, { 0, 1 }, { 0, -1 } }) do
        local _, _, _, na = kagero.get_pixel(x + d[1], y + d[2])
        if na and na > 0 then
          targets[#targets + 1] = { x, y }
          break
        end
      end
    end
  end)
  -- Paint after looking, so new outline pixels don't grow the outline further.
  for _, p in ipairs(targets) do
    kagero.set_pixel(p[1], p[2], pr, pg, pb, pa)
  end
  kagero.message("Outlined with " .. #targets .. " pixels")
end

return commands
