-- Pixel Cleanup: a Kagero plugin (API 1).
--
-- Every command first looks at the image and then paints, so pixels it changes
-- don't affect what it decides for their neighbors.

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

-- Alpha at (x, y); 0 outside the image.
local function alpha(x, y)
  local _, _, _, a = kagero.get_pixel(x, y)
  return a or 0
end

local SIDES = { { 1, 0 }, { -1, 0 }, { 0, 1 }, { 0, -1 } }
local AROUND = {
  { -1, -1 }, { 0, -1 }, { 1, -1 },
  { -1, 0 }, { 1, 0 },
  { -1, 1 }, { 0, 1 }, { 1, 1 },
}

local function plural(n, word)
  if n == 1 then
    return "1 " .. word
  end
  return n .. " " .. word .. "s"
end

local commands = {}

-- Single pixels with nothing around them (not even diagonally): usually stray clicks.
commands["remove-strays"] = function()
  local strays = {}
  each_pixel(function(x, y, r, g, b, a)
    if a > 0 then
      for _, d in ipairs(AROUND) do
        if alpha(x + d[1], y + d[2]) > 0 then
          return
        end
      end
      strays[#strays + 1] = { x, y }
    end
  end)
  for _, p in ipairs(strays) do
    kagero.set_pixel(p[1], p[2], 0, 0, 0, 0)
  end
  kagero.message("Removed " .. plural(#strays, "stray pixel"))
end

-- Half-transparent pixels (from scaled or imported images) become fully opaque or
-- fully transparent, so edges stay crisp.
commands["harden-edges"] = function()
  local count = 0
  local changes = {}
  each_pixel(function(x, y, r, g, b, a)
    if a > 0 and a < 255 then
      if a >= 128 then
        changes[#changes + 1] = { x, y, r, g, b, 255 }
      else
        changes[#changes + 1] = { x, y, 0, 0, 0, 0 }
      end
    end
  end)
  for _, c in ipairs(changes) do
    kagero.set_pixel(c[1], c[2], c[3], c[4], c[5], c[6])
    count = count + 1
  end
  kagero.message("Hardened " .. plural(count, "pixel"))
end

-- A shadow one pixel down and right, in the secondary color, only on empty pixels.
commands["drop-shadow"] = function()
  local sr, sg, sb, sa = kagero.secondary()
  local shadow = {}
  each_pixel(function(x, y, r, g, b, a)
    if a == 0 and alpha(x - 1, y - 1) > 0 then
      shadow[#shadow + 1] = { x, y }
    end
  end)
  for _, p in ipairs(shadow) do
    kagero.set_pixel(p[1], p[2], sr, sg, sb, sa)
  end
  kagero.message("Shadow: " .. plural(#shadow, "pixel"))
end

-- Repaints the outermost pixels of every shape (those next to an empty pixel or the
-- image edge) with the primary color. Unlike an outline, the shape doesn't grow.
commands["inner-outline"] = function()
  local pr, pg, pb, pa = kagero.primary()
  local edge = {}
  each_pixel(function(x, y, r, g, b, a)
    if a > 0 then
      for _, d in ipairs(SIDES) do
        if alpha(x + d[1], y + d[2]) == 0 then
          edge[#edge + 1] = { x, y }
          return
        end
      end
    end
  end)
  for _, p in ipairs(edge) do
    kagero.set_pixel(p[1], p[2], pr, pg, pb, pa)
  end
  kagero.message("Inner outline: " .. plural(#edge, "pixel"))
end

return commands
