extends SceneTree
func _initialize():
 for cls in ["StatusIndicator", "NativeMenu", "DisplayServer", "Environment", "Viewport"]:
  print("CLASS ", cls)
  if cls in ["StatusIndicator", "Environment", "Viewport"]:
   for p in ClassDB.class_get_property_list(cls, true):
    if cls == "StatusIndicator" or "ssao" in p.name or "msaa" in p.name or "screen_space" in p.name or "scaling_3d" in p.name: print("PROP ", p)
  if cls in ["StatusIndicator", "NativeMenu", "DisplayServer"]:
   for m in ClassDB.class_get_method_list(cls,true):
    if cls != "DisplayServer" or "status_indicator" in m.name or "feature" in m.name: print("METHOD ",m)
  if cls == "StatusIndicator": print("SIGNALS ",ClassDB.class_get_signal_list(cls,true))
 print("RENDERER ", RenderingServer.get_current_rendering_method())
 quit()
