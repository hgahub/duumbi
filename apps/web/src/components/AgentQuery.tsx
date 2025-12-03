import { useState } from 'react';
import { useTranslation } from 'react-i18next';

// Mock icons - replace with actual icons from your icon library (e.g., Lucide, Heroicons)
// Assuming Lucide or similar for now based on typical React stacks, but using SVGs for standalone safety if needed.
// For now, I'll use simple SVG placeholders to ensure it renders without external deps issues,
// but in a real app, I'd import from 'lucide-react' or similar if available.

const PlusIcon = () => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width="20"
    height="20"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M5 12h14" />
    <path d="M12 5v14" />
  </svg>
);

const MicIcon = () => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width="20"
    height="20"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
    <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
    <line x1="12" y1="19" x2="12" y2="23" />
    <line x1="8" y1="23" x2="16" y2="23" />
  </svg>
);

const SparklesIcon = () => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width="20"
    height="20"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275L12 3Z" />
  </svg>
);

const ChevronDownIcon = () => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="m6 9 6 6 6-6" />
  </svg>
);

export const AgentQuery = () => {
  const { t } = useTranslation();
  const [query, setQuery] = useState('');

  const handleSearch = () => {
    console.log('Mock Search:', query);
  };

  const handleUpload = () => {
    console.log('Mock Upload Image');
  };

  const handleVoice = () => {
    console.log('Mock Voice Search');
  };

  const handleWizard = () => {
    console.log('Mock Prompt Wizard');
  };

  return (
    <div className="w-full max-w-3xl mx-auto mt-8">
      <div className="relative bg-white dark:bg-gray-800/50 backdrop-blur-sm border border-gray-200 dark:border-gray-700 rounded-2xl shadow-lg transition-all focus-within:ring-2 focus-within:ring-blue-500/50 focus-within:border-blue-500/50">
        <div className="p-4">
          <textarea
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                handleSearch();
              }
            }}
            placeholder={t('Ask Duumbi')}
            className="w-full bg-transparent text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 resize-none outline-none text-lg min-h-[60px]"
            rows={2}
          />
        </div>

        <div className="flex items-center justify-between px-4 pb-3">
          <div className="flex items-center gap-3">
            <button
              onClick={handleUpload}
              className="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-700/50 rounded-full transition-colors"
              title={t('Upload Image')}
            >
              <PlusIcon />
            </button>

            <button className="flex items-center gap-2 px-3 py-1.5 text-sm text-gray-600 dark:text-gray-300 bg-gray-100 dark:bg-gray-700/30 hover:bg-gray-200 dark:hover:bg-gray-700/50 rounded-full transition-colors">
              <span>{t('Tools')}</span>
              <ChevronDownIcon />
            </button>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={handleWizard}
              className="p-2 text-gray-500 dark:text-gray-400 hover:text-purple-600 dark:hover:text-purple-400 hover:bg-gray-100 dark:hover:bg-gray-700/50 rounded-full transition-colors"
              title={t('Prompt Wizard')}
            >
              <SparklesIcon />
            </button>
            <button
              onClick={handleVoice}
              className="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-700/50 rounded-full transition-colors"
              title={t('Voice Search')}
            >
              <MicIcon />
            </button>
          </div>
        </div>
      </div>

      {/* Quick Action Chips */}
      <div className="flex flex-wrap justify-center gap-3 mt-6">
        {[
          'Create Image',
          'Write anything',
          'Build an idea',
          'Deep Research',
        ].map((action) => (
          <button
            key={action}
            className="px-4 py-2 text-sm text-gray-600 dark:text-gray-300 bg-white dark:bg-gray-800/40 hover:bg-gray-50 dark:hover:bg-gray-800/60 border border-gray-200 dark:border-gray-700/50 rounded-xl transition-colors shadow-sm"
          >
            {t(action)}
          </button>
        ))}
      </div>
    </div>
  );
};

export default AgentQuery;
