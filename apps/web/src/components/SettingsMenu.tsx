import { Menu, Transition } from '@headlessui/react';
import {
  ChatBubbleLeftRightIcon,
  CheckIcon,
  ChevronRightIcon,
  ComputerDesktopIcon,
  CreditCardIcon,
  MoonIcon,
  QuestionMarkCircleIcon,
  SunIcon,
  UserIcon,
} from '@heroicons/react/24/outline';
import React, { Fragment, useEffect, useRef, useState } from 'react';
import { useTheme } from '@duumbi/ui-components';
import { useTranslation } from 'react-i18next';

interface SettingsMenuProps {
  trigger: React.ReactNode;
  onNavigate: (view: 'home' | 'preferences') => void;
}

export default function SettingsMenu({ trigger, onNavigate }: SettingsMenuProps) {
  const { t } = useTranslation();
  const { theme, setTheme } = useTheme();
  const [activeSubmenu, setActiveSubmenu] = useState<string | null>(null);

  // Close submenus when clicking outside
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setActiveSubmenu(null);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, []);

  const handleSubmenuEnter = (menuName: string) => {
    setActiveSubmenu(menuName);
  };

  return (
    <div className="relative" ref={menuRef}>
      <Menu as="div" className="relative inline-block text-left w-full">
        <Menu.Button as="div" className="w-full">
          {trigger}
        </Menu.Button>

        <Transition
          as={Fragment}
          enter="transition ease-out duration-100"
          enterFrom="transform opacity-0 scale-95"
          enterTo="transform opacity-100 scale-100"
          leave="transition ease-in duration-75"
          leaveFrom="transform opacity-100 scale-100"
          leaveTo="transform opacity-0 scale-95"
        >
          <Menu.Items className="absolute bottom-full left-0 mb-2 w-64 origin-bottom-left rounded-md bg-white dark:bg-higashi-kashmirblue-800 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none z-50">
            <div className="px-1 py-1">
              {/* Preferences */}
              <Menu.Item>
                {({ active }) => (
                  <button
                    onClick={() => {
                      onNavigate('preferences');
                      setActiveSubmenu(null);
                    }}
                    onMouseEnter={() => setActiveSubmenu(null)}
                    className={`${
                      active
                        ? 'bg-gray-100 dark:bg-higashi-kashmirblue-700 text-gray-900 dark:text-white'
                        : 'text-gray-700 dark:text-gray-200'
                    } group flex w-full items-center rounded-md px-2 py-2 text-sm`}
                  >
                    <UserIcon className="mr-2 h-5 w-5" aria-hidden="true" />
                    {t('Preferences')}
                  </button>
                )}
              </Menu.Item>

              {/* Subscription Management */}
              <Menu.Item>
                {({ active }) => (
                  <button
                    onMouseEnter={() => setActiveSubmenu(null)}
                    className={`${
                      active
                        ? 'bg-gray-100 dark:bg-higashi-kashmirblue-700 text-gray-900 dark:text-white'
                        : 'text-gray-700 dark:text-gray-200'
                    } group flex w-full items-center rounded-md px-2 py-2 text-sm`}
                  >
                    <CreditCardIcon
                      className="mr-2 h-5 w-5"
                      aria-hidden="true"
                    />
                    {t('Subscription Management')}
                  </button>
                )}
              </Menu.Item>
            </div>

            <div className="px-1 py-1">
              {/* Theme Submenu Trigger */}
              <div
                className="relative"
                onMouseEnter={() => handleSubmenuEnter('theme')}
                onMouseLeave={() => {}} // Keep open to allow interaction
              >
                <Menu.Item>
                  {({ active }) => (
                    <button
                      className={`${
                        active || activeSubmenu === 'theme'
                          ? 'bg-gray-100 dark:bg-higashi-kashmirblue-700 text-gray-900 dark:text-white'
                          : 'text-gray-700 dark:text-gray-200'
                      } group flex w-full items-center justify-between rounded-md px-2 py-2 text-sm`}
                    >
                      <div className="flex items-center">
                        <SunIcon className="mr-2 h-5 w-5" aria-hidden="true" />
                        {t('Theme')}
                      </div>
                      <ChevronRightIcon
                        className="h-4 w-4"
                        aria-hidden="true"
                      />
                    </button>
                  )}
                </Menu.Item>

                {/* Theme Submenu */}
                {activeSubmenu === 'theme' && (
                  <div className="absolute left-full bottom-0 ml-1 w-48 origin-top-right rounded-md bg-white dark:bg-higashi-kashmirblue-800 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none">
                    <div className="px-1 py-1">
                      <button
                        onClick={() => setTheme('system')}
                        className={`${
                          theme === 'system'
                            ? 'bg-gray-100 dark:bg-higashi-kashmirblue-700 text-gray-900 dark:text-white'
                            : 'text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-higashi-kashmirblue-700'
                        } group flex w-full items-center justify-between rounded-md px-2 py-2 text-sm`}
                      >
                        <div className="flex items-center">
                          <ComputerDesktopIcon className="mr-2 h-5 w-5" />
                          {t('System')}
                        </div>
                        {theme === 'system' && (
                          <CheckIcon className="h-4 w-4" />
                        )}
                      </button>
                      <button
                        onClick={() => setTheme('light')}
                        className={`${
                          theme === 'light'
                            ? 'bg-gray-100 dark:bg-higashi-kashmirblue-700 text-gray-900 dark:text-white'
                            : 'text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-higashi-kashmirblue-700'
                        } group flex w-full items-center justify-between rounded-md px-2 py-2 text-sm`}
                      >
                        <div className="flex items-center">
                          <SunIcon className="mr-2 h-5 w-5" />
                          {t('Light')}
                        </div>
                        {theme === 'light' && <CheckIcon className="h-4 w-4" />}
                      </button>
                      <button
                        onClick={() => setTheme('dark')}
                        className={`${
                          theme === 'dark'
                            ? 'bg-gray-100 dark:bg-higashi-kashmirblue-700 text-gray-900 dark:text-white'
                            : 'text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-higashi-kashmirblue-700'
                        } group flex w-full items-center justify-between rounded-md px-2 py-2 text-sm`}
                      >
                        <div className="flex items-center">
                          <MoonIcon className="mr-2 h-5 w-5" />
                          {t('Dark')}
                        </div>
                        {theme === 'dark' && <CheckIcon className="h-4 w-4" />}
                      </button>
                    </div>
                  </div>
                )}
              </div>
            </div>

            <div className="px-1 py-1">
              {/* Feedback */}
              <Menu.Item>
                {({ active }) => (
                  <button
                    onMouseEnter={() => setActiveSubmenu(null)}
                    className={`${
                      active
                        ? 'bg-gray-100 dark:bg-higashi-kashmirblue-700 text-gray-900 dark:text-white'
                        : 'text-gray-700 dark:text-gray-200'
                    } group flex w-full items-center rounded-md px-2 py-2 text-sm`}
                  >
                    <ChatBubbleLeftRightIcon
                      className="mr-2 h-5 w-5"
                      aria-hidden="true"
                    />
                    {t('Feedback')}
                  </button>
                )}
              </Menu.Item>

              {/* Help Submenu Trigger */}
              <div
                className="relative"
                onMouseEnter={() => handleSubmenuEnter('help')}
                onMouseLeave={() => {}}
              >
                <Menu.Item>
                  {({ active }) => (
                    <button
                      className={`${
                        active || activeSubmenu === 'help'
                          ? 'bg-gray-100 dark:bg-higashi-kashmirblue-700 text-gray-900 dark:text-white'
                          : 'text-gray-700 dark:text-gray-200'
                      } group flex w-full items-center justify-between rounded-md px-2 py-2 text-sm`}
                    >
                      <div className="flex items-center">
                        <QuestionMarkCircleIcon
                          className="mr-2 h-5 w-5"
                          aria-hidden="true"
                        />
                        {t('Help')}
                      </div>
                      <ChevronRightIcon
                        className="h-4 w-4"
                        aria-hidden="true"
                      />
                    </button>
                  )}
                </Menu.Item>

                {/* Help Submenu */}
                {activeSubmenu === 'help' && (
                  <div className="absolute left-full bottom-0 ml-1 w-48 origin-top-right rounded-md bg-white dark:bg-higashi-kashmirblue-800 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none">
                    <div className="px-1 py-1">
                      {[
                        'Help Center',
                        'FAQ',
                        'Contact',
                        'About Us',
                        'Privacy Policy',
                        'Terms of Service',
                        'Cookie Settings',
                      ].map((item) => (
                        <button
                          key={item}
                          className="text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-higashi-kashmirblue-700 group flex w-full items-center rounded-md px-2 py-2 text-sm"
                        >
                          {t(item)}
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
          </Menu.Items>
        </Transition>
      </Menu>
    </div>
  );
}
