extends RefCounted
## Coordinates come directly from DisplayServer, including negative origins.
## Keep the screen union: a bounding rectangle would include invisible gaps.
var areas: Array[Rect2] = []
var extent := Vector2(320, 248) # window width and distance from origin to feet

func screen_at(point: Vector2) -> int:
	var best := 0
	var distance := INF
	for i in areas.size():
		if areas[i].has_point(point):
			return i
		var nearest := point.clamp(areas[i].position, areas[i].end)
		var d := point.distance_squared_to(nearest)
		if d < distance:
			distance = d
			best = i
	return best

func current(at: Vector2) -> int:
	return screen_at(at + Vector2(extent.x * 0.5, extent.y - 1))

func bounds_for(at: Vector2) -> Rect2:
	if areas.is_empty():
		return Rect2(0, 0, 1600, 800)
	var area := areas[current(at)]
	var low := area.position
	var high := (area.end - extent).max(low)
	var feet := at.y + extent.y
	for other in areas:
		# Open a shared edge only where the complete visible height fits.
		# A higher neighbour's floor remains a step, not a teleport upwards.
		if at.y >= other.position.y and feet <= other.end.y + 0.01:
			if absf(area.end.x - other.position.x) < 0.5:
				high.x = maxf(high.x, other.end.x - extent.x)
			if absf(area.position.x - other.end.x) < 0.5:
				low.x = minf(low.x, other.position.x)
		if at.x >= other.position.x and at.x + extent.x <= other.end.x:
			if absf(area.end.y - other.position.y) < 0.5:
				high.y = maxf(high.y, other.end.y - extent.y)
			if absf(area.position.y - other.end.y) < 0.5:
				low.y = minf(low.y, other.position.y)
	return Rect2(low, high - low)

func clamp_origin(at: Vector2) -> Vector2:
	# In a hole or on a removed display, recover fully onto the nearest screen.
	var anchor := at + Vector2(extent.x * 0.5, extent.y - 1)
	var area := areas[current(at)] if not areas.is_empty() else Rect2(0, 0, 1920, 1080)
	var bounds := bounds_for(at)
	if not area.has_point(anchor):
		bounds = Rect2(area.position, (area.size - extent).max(Vector2.ZERO))
	return at.clamp(bounds.position, bounds.end)
