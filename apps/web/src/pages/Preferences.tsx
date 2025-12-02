import { Listbox, Transition } from '@headlessui/react';
import { CheckIcon, ChevronUpDownIcon } from '@heroicons/react/20/solid';
import { Fragment } from 'react';
import { useTranslation } from 'react-i18next';

const languages = [
  { id: 'en', name: 'English' },
  { id: 'de', name: 'Deutsch' },
  { id: 'hu', name: 'Magyar' },
  { id: 'es', name: 'Español' },
  { id: 'pl', name: 'Polski' },
  { id: 'it', name: 'Italiano' },
  { id: 'default', name: 'Default' }, // Represents system/fallback
];

export default function Preferences({ onBack }: { onBack: () => void }) {
  const { t, i18n } = useTranslation();

  const currentLanguageId = i18n.resolvedLanguage || 'en';
  // If resolved language is not in our list (or is a variant), fallback to 'en' or find best match.
  // For simplicity, we'll try to match the prefix or default to 'en'.
  // However, for the dropdown display, we want to show what's selected.

  const selectedLanguage =
    languages.find((l) => l.id === currentLanguageId) ||
    languages.find((l) => currentLanguageId.startsWith(l.id)) ||
    languages[0];

  const handleLanguageChange = (lang: { id: string; name: string }) => {
    if (lang.id === 'default') {
      // Reset to detected language or default fallback
      i18n.changeLanguage();
    } else {
      i18n.changeLanguage(lang.id);
    }
  };

  return (
    <div className="w-full max-w-4xl mx-auto p-6">
      <button
        onClick={onBack}
        className="mb-4 flex items-center text-sm text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 transition-colors"
      >
        &lt; {t('Back')}
      </button>
      <h1 className="text-2xl font-bold text-gray-900 dark:text-white mb-6">
        {t('Preferences')}
      </h1>

      <div className="border-t border-gray-200 dark:border-higashi-kashmirblue-700 pt-6">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-900 dark:text-white">
              {t('Language')}
            </label>
            <p className="text-sm text-gray-500 dark:text-gray-400">
              {t('The language used in the user interface')}
            </p>
          </div>

          <div className="w-full md:w-64">
            <Listbox value={selectedLanguage} onChange={handleLanguageChange}>
              <div className="relative mt-1">
                <Listbox.Button className="relative w-full cursor-default rounded-lg bg-white dark:bg-higashi-kashmirblue-800 py-2 pl-3 pr-10 text-left shadow-md focus:outline-none focus-visible:border-indigo-500 focus-visible:ring-2 focus-visible:ring-white/75 focus-visible:ring-offset-2 focus-visible:ring-offset-orange-300 sm:text-sm border border-gray-200 dark:border-higashi-kashmirblue-700 text-gray-900 dark:text-white">
                  <span className="block truncate">
                    {selectedLanguage.name}
                  </span>
                  <span className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
                    <ChevronUpDownIcon
                      className="h-5 w-5 text-gray-400"
                      aria-hidden="true"
                    />
                  </span>
                </Listbox.Button>
                <Transition
                  as={Fragment}
                  leave="transition ease-in duration-100"
                  leaveFrom="opacity-100"
                  leaveTo="opacity-0"
                >
                  <Listbox.Options className="absolute mt-1 max-h-60 w-full overflow-auto rounded-md bg-white dark:bg-higashi-kashmirblue-800 py-1 text-base shadow-lg ring-1 ring-black/5 focus:outline-none sm:text-sm z-10">
                    {languages.map((lang, langIdx) => (
                      <Listbox.Option
                        key={langIdx}
                        className={({ active }) =>
                          `relative cursor-default select-none py-2 pl-10 pr-4 ${
                            active
                              ? 'bg-gray-100 dark:bg-higashi-kashmirblue-700 text-gray-900 dark:text-white'
                              : 'text-gray-900 dark:text-white'
                          }`
                        }
                        value={lang}
                      >
                        {({ selected }) => (
                          <>
                            <span
                              className={`block truncate ${
                                selected ? 'font-medium' : 'font-normal'
                              }`}
                            >
                              {lang.name}
                            </span>
                            {selected ? (
                              <span className="absolute inset-y-0 left-0 flex items-center pl-3 text-indigo-600 dark:text-indigo-400">
                                <CheckIcon
                                  className="h-5 w-5"
                                  aria-hidden="true"
                                />
                              </span>
                            ) : null}
                          </>
                        )}
                      </Listbox.Option>
                    ))}
                  </Listbox.Options>
                </Transition>
              </div>
            </Listbox>
          </div>
        </div>
      </div>
    </div>
  );
}
