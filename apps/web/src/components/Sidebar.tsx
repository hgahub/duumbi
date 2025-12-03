import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import SettingsMenu from './SettingsMenu';

interface SidebarProps {
  isOpen: boolean;
  onClose: () => void;
  onNavigate: (view: 'home' | 'preferences') => void;
  onNewChat?: () => void;
}

export default function Sidebar({
  isOpen,
  onClose,
  onNavigate,
  onNewChat,
}: SidebarProps) {
  const { t } = useTranslation();
  const [isCollapsed, setIsCollapsed] = useState(false);

  const toggleCollapse = () => {
    setIsCollapsed(!isCollapsed);
  };

  // Mock recent chats data
  const recentChats = [
    'Project Plan Brainstorm',
    'Marketing Email Draft',
    'Code Debugging Session',
  ];

  return (
    <>
      {/* Mobile overlay */}
      {isOpen && (
        <div
          className="fixed inset-0 bg-black bg-opacity-50 z-10 md:hidden"
          onClick={onClose}
        />
      )}

      {/* Sidebar */}
      <aside
        className={`
          group sidebar
          ${isCollapsed ? 'is-collapsed w-20' : 'w-64'}
          ${isOpen ? '' : 'sidebar-hidden'}
          bg-higashi-concrete-200 dark:bg-higashi-kashmirblue-800
          flex flex-col flex-shrink-0
          fixed md:relative inset-y-0 left-0 z-20 md:z-auto
          shadow-lg md:shadow-none
          border-r border-gray-200 dark:border-higashi-kashmirblue-700
        `}
      >
        {/* Header */}
        <div className="p-4 flex-shrink-0 flex items-center justify-between">
          <div
            className={`flex items-center justify-center flex-1 ${isCollapsed ? 'hidden' : ''}`}
          >
            <img
              src="/DuumbiLogoLight.svg"
              alt="Duumbi Logo"
              className="h-12 dark:hidden"
            />
            <img
              src="/DuumbiLogoDark.svg"
              alt="Duumbi Logo"
              className="h-12 hidden dark:block"
            />
          </div>
          <button
            type="button"
            onClick={toggleCollapse}
            aria-label={isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            title={isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            className="hidden md:block p-2 rounded-md hover:bg-gray-100 dark:hover:bg-higashi-kashmirblue-700"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className={`h-6 w-6 transition-transform duration-300 ${
                isCollapsed ? 'rotate-180' : ''
              }`}
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 19l-7-7 7-7"
              />
            </svg>
          </button>
        </div>

        {/* Navigation */}
        <nav className="p-4 space-y-2 flex-grow overflow-y-auto">
          {/* New Chat */}
          <button
            onClick={onNewChat}
            className="w-full flex items-center p-2 text-base font-normal text-gray-900 dark:text-white rounded-lg hover:bg-gray-100 dark:hover:bg-higashi-kashmirblue-700"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="h-6 w-6 text-gray-500 dark:text-higashi-kashmirblue-300"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
              />
            </svg>
            <span className={`ml-3 ${isCollapsed ? 'hidden' : ''}`}>
              {t('New Chat')}
            </span>
          </button>

          {/* Post New Listing */}
          <a
            href="#"
            className="flex items-center p-2 text-base font-normal text-gray-900 dark:text-white rounded-lg hover:bg-gray-100 dark:hover:bg-higashi-kashmirblue-700"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="h-6 w-6 text-gray-500 dark:text-higashi-kashmirblue-300"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M12 9v3m0 0v3m0-3h3m-3 0H9m12 0a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <span className={`ml-3 ${isCollapsed ? 'hidden' : ''}`}>
              {t('Post New Listing')}
            </span>
          </a>

          {/* My Listings */}
          <a
            href="#"
            className="flex items-center p-2 text-base font-normal text-gray-900 dark:text-white rounded-lg hover:bg-gray-100 dark:hover:bg-higashi-kashmirblue-700"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="h-6 w-6 text-gray-500 dark:text-higashi-kashmirblue-300"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M4 6h16M4 10h16M4 14h16M4 18h16"
              />
            </svg>
            <span className={`ml-3 ${isCollapsed ? 'hidden' : ''}`}>
              {t('My Listings')}
            </span>
          </a>

          {/* Recent Chats */}
          {!isCollapsed && (
            <div className="pt-4 mt-4 space-y-2 border-t border-gray-200 dark:border-higashi-kashmirblue-700">
              <h2 className="px-2 text-xs font-semibold text-gray-600 dark:text-higashi-kashmirblue-200 uppercase tracking-wider">
                {t('Recent Chats')}
              </h2>
              {recentChats.map((chat, index) => (
                <a
                  key={index}
                  href="#"
                  className="block p-2 text-sm truncate rounded-lg hover:bg-gray-100 dark:hover:bg-higashi-kashmirblue-700"
                >
                  {chat}
                </a>
              ))}
            </div>
          )}
        </nav>

        {/* Settings at bottom */}
        <div
          className={`p-4 border-t border-gray-200 dark:border-higashi-kashmirblue-700 ${
            isCollapsed ? 'flex justify-center' : ''
          }`}
        >
          <SettingsMenu
            onNavigate={onNavigate}
            trigger={
              <button className="flex w-full items-center p-2 text-base font-normal text-gray-900 dark:text-white rounded-lg hover:bg-gray-100 dark:hover:bg-higashi-kashmirblue-700">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  className="h-6 w-6 text-gray-500 dark:text-higashi-kashmirblue-300"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                  />
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                  />
                </svg>
                <span className={`ml-3 ${isCollapsed ? 'hidden' : ''}`}>
                  {t('Settings')}
                </span>
              </button>
            }
          />
        </div>
      </aside>
    </>
  );
}
