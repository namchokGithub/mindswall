# MindsWall design context

MindsWall is a quiet desktop utility for managing a personal wallpaper collection.
The interface should stay out of the way of the images, use clear English labels,
and make the current slideshow state easy to understand at a glance.

- Use a dark charcoal workspace with muted lavender accents, soft borders, and
  generous spacing, following the supplied overview prototype.
- Use the system UI font and consistent shared controls; reserve bright accents
  for the primary action, focus indicators, and active navigation.
- Use the MindsWall mascot from `src/images/` for the app identity and icons.
- Organize the application into Overview, Folders, Gallery, Monitors, History,
  Profiles, Editor, and Settings. Show loading,
  empty, error, and saved states explicitly; do not present planned features as
  working controls.
- Support keyboard navigation, visible focus, accessible labels, and reduced motion.
- Show transient feedback in a floating snackbar scoped to the active page;
  discard feedback from an earlier page when navigation changes.
- Keep image paths and settings local to the device.
