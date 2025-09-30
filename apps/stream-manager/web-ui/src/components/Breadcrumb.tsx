import { Link, useLocation } from "react-router-dom";
import { ChevronRightIcon, HomeIcon } from "@heroicons/react/20/solid";
import { navigationItems } from "../utils/navigation.ts";

export default function Breadcrumb() {
  const location = useLocation();
  const pathSegments = location.pathname.split("/").filter(Boolean);

  if (pathSegments.length === 0) {
    return null; // Don't show breadcrumb on homepage
  }

  const breadcrumbs = pathSegments.map((segment, index) => {
    const path = "/" + pathSegments.slice(0, index + 1).join("/");
    const navItem = navigationItems.find((item) => item.path === path);
    const name = navItem?.name || segment.charAt(0).toUpperCase() + segment.slice(1);

    return {
      name,
      path,
      current: index === pathSegments.length - 1,
    };
  });

  return (
    <nav className="flex" aria-label="Breadcrumb">
      <ol className="flex items-center space-x-4">
        <li>
          <div>
            <Link
              to="/"
              className="text-gray-400 hover:text-gray-500 dark:text-gray-500 dark:hover:text-gray-400"
            >
              <HomeIcon className="h-5 w-5 flex-shrink-0" aria-hidden="true" />
              <span className="sr-only">Home</span>
            </Link>
          </div>
        </li>
        {breadcrumbs.map((item) => (
          <li key={item.path}>
            <div className="flex items-center">
              <ChevronRightIcon
                className="h-5 w-5 flex-shrink-0 text-gray-400 dark:text-gray-500"
                aria-hidden="true"
              />
              {item.current
                ? (
                  <span className="ml-4 text-sm font-medium text-gray-500 dark:text-gray-400">
                    {item.name}
                  </span>
                )
                : (
                  <Link
                    to={item.path}
                    className="ml-4 text-sm font-medium text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
                  >
                    {item.name}
                  </Link>
                )}
            </div>
          </li>
        ))}
      </ol>
    </nav>
  );
}
