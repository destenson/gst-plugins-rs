import { useEffect } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { navigationItems } from "../utils/navigation.ts";

export function useKeyboardNavigation(
  sidebarOpen: boolean,
  setSidebarOpen: (open: boolean) => void,
) {
  const navigate = useNavigate();
  const location = useLocation();

  useEffect(() => {
    function handleKeyPress(event: KeyboardEvent) {
      // Toggle sidebar with Ctrl+B or Cmd+B
      if ((event.ctrlKey || event.metaKey) && event.key === "b") {
        event.preventDefault();
        setSidebarOpen(!sidebarOpen);
      }

      // Navigate with Alt+number keys
      if (event.altKey && event.key >= "1" && event.key <= "9") {
        event.preventDefault();
        const index = parseInt(event.key) - 1;
        if (index < navigationItems.length) {
          navigate(navigationItems[index].path);
        }
      }

      // Navigate with arrow keys when sidebar is focused
      if (sidebarOpen && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
        const currentIndex = navigationItems.findIndex(
          (item) => item.path === location.pathname,
        );

        if (currentIndex !== -1) {
          event.preventDefault();
          let nextIndex = currentIndex;

          if (event.key === "ArrowUp") {
            nextIndex = currentIndex > 0 ? currentIndex - 1 : navigationItems.length - 1;
          } else {
            nextIndex = currentIndex < navigationItems.length - 1 ? currentIndex + 1 : 0;
          }

          navigate(navigationItems[nextIndex].path);
        }
      }

      // Close sidebar with Escape
      if (event.key === "Escape" && sidebarOpen) {
        setSidebarOpen(false);
      }
    }

    globalThis.addEventListener("keydown", handleKeyPress);
    return () => globalThis.removeEventListener("keydown", handleKeyPress);
  }, [navigate, location, sidebarOpen, setSidebarOpen]);
}
