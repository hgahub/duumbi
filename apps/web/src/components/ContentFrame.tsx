import { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import AgentQuery from './AgentQuery';

interface ContentFrameProps {
  children?: ReactNode;
}

export default function ContentFrame({ children }: ContentFrameProps) {
  const { t } = useTranslation();
  return (
    <main className="flex-1 p-4 md:p-6 lg:p-8 overflow-y-auto flex flex-col">
      <div className="max-w-4xl w-full mx-auto flex flex-col flex-grow">
        {children || (
          <div className="flex items-center justify-center flex-grow">
            <div className="text-center w-full">
              <h2 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-higashi-kashmirblue-200 to-higashi-kashmirblue-100 mb-8">
                {t('How can I help you today?')}
              </h2>
              <AgentQuery />
            </div>
          </div>
        )}
      </div>
    </main>
  );
}
