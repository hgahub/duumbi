import { useTranslation } from 'react-i18next';

interface TopMenuProps {
  onMenuToggle: () => void;
}

export default function TopMenu({ onMenuToggle }: TopMenuProps) {
  const { t } = useTranslation();
  // Placeholder for authentication state
  const isAuthenticated = false;

  return (
    <header className="flex items-center justify-between p-4 bg-higashi-concrete-200 dark:bg-higashi-kashmirblue-800 border-b border-gray-200 dark:border-higashi-kashmirblue-700 md:justify-end">
      {/* Mobile menu button (icon-only) - add accessible name and type to satisfy axe */}
      <button
        type="button"
        onClick={onMenuToggle}
        aria-label="Open navigation menu"
        title="Open navigation menu"
        className="md:hidden p-2 bg-gray-200 dark:bg-higashi-kashmirblue-800 rounded-md"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          className="h-6 w-6"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M4 6h16M4 12h16m-7 6h7"
          />
        </svg>
      </button>

      {/* Action buttons */}
      <div className="flex items-center space-x-4">
        {isAuthenticated ? (
          <>
            {/* Notifications (icon-only) - add aria-label/title and type */}
            <button
              type="button"
              aria-label="View notifications"
              title="View notifications"
              className="p-2 rounded-full hover:bg-gray-100 dark:hover:bg-higashi-kashmirblue-700"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="h-6 w-6"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
                />
              </svg>
            </button>

            {/* User Profile - button contains image with alt text, but add explicit aria-label/title for clarity */}
            <button
              type="button"
              aria-label="Open user menu"
              title="Open user menu"
            >
              <img
                src="https://placehold.co/32x32/E2E8F0/4A5568?text=U"
                className="rounded-full h-8 w-8"
                alt="User Avatar"
              />
            </button>
          </>
        ) : (
          <>
            <button
              type="button"
              className="text-gray-700 dark:text-higashi-concrete-200 hover:text-gray-900 dark:hover:text-white font-medium px-3 py-2 rounded-md transition-colors border border-higashi-kashmirblue-500"
            >
              {t('Sign in')}
            </button>
            <button
              type="button"
              className="bg-gradient-to-r from-higashi-kashmirblue-500 to-higashi-kashmirblue-500 hover:opacity-90 text-white px-4 py-2 rounded-md font-medium transition-colors shadow-sm"
            >
              {t('Sign up')}
            </button>
          </>
        )}
      </div>
    </header>
  );
}
