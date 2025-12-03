import React, { useState } from 'react';
import { supabase } from '../lib/supabase';
import { useTranslation } from 'react-i18next';

export default function Login() {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [isSignUp, setIsSignUp] = useState(false);
  const [message, setMessage] = useState<{ type: 'error' | 'success'; text: string } | null>(null);

  const handleAuth = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setMessage(null);

    try {
      if (isSignUp) {
        const { error } = await supabase.auth.signUp({
          email,
          password,
        });
        if (error) throw error;
        setMessage({ type: 'success', text: 'Check your email for the confirmation link!' });
      } else {
        const { error } = await supabase.auth.signInWithPassword({
          email,
          password,
        });
        if (error) throw error;
        // Navigation will be handled by AuthContext state change in App.tsx
      }
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : 'An error occurred';
      setMessage({ type: 'error', text: message });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-gray-100 dark:bg-higashi-kashmirblue-900 px-4 py-12 sm:px-6 lg:px-8 transition-colors duration-200">
      <div className="w-full max-w-md space-y-8 bg-white dark:bg-higashi-kashmirblue-800 p-10 rounded-2xl shadow-2xl border border-gray-200 dark:border-higashi-kashmirblue-700 transition-colors duration-200">
        <div>
          <h2 className="mt-2 text-center text-3xl font-bold tracking-tight text-gray-900 dark:text-white">
            {isSignUp ? t('Create your account') : t('Sign in to Duumbi')}
          </h2>
          <p className="mt-2 text-center text-sm text-gray-600 dark:text-gray-400">
            {isSignUp ? t('Start your journey with us') : t('Welcome back')}
          </p>
        </div>
        <form className="mt-8 space-y-6" onSubmit={handleAuth}>
          <div className="space-y-4">
            <div>
              <label htmlFor="email-address" className="block text-sm font-medium leading-6 text-gray-900 dark:text-gray-300">
                {t('Email address')}
              </label>
              <div className="mt-2">
                <input
                  id="email-address"
                  name="email"
                  type="email"
                  autoComplete="email"
                  required
                  className="block w-full rounded-md border-0 bg-gray-50 dark:bg-higashi-kashmirblue-900/50 py-2.5 text-gray-900 dark:text-white shadow-sm ring-1 ring-inset ring-gray-300 dark:ring-higashi-kashmirblue-600 placeholder:text-gray-400 dark:placeholder:text-gray-500 focus:ring-2 focus:ring-inset focus:ring-higashi-kashmirblue-500 sm:text-sm sm:leading-6 px-3 transition-colors duration-200"
                  placeholder="name@example.com"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
              </div>
            </div>
            <div>
              <label htmlFor="password" className="block text-sm font-medium leading-6 text-gray-900 dark:text-gray-300">
                {t('Password')}
              </label>
              <div className="mt-2">
                <input
                  id="password"
                  name="password"
                  type="password"
                  autoComplete="current-password"
                  required
                  className="block w-full rounded-md border-0 bg-gray-50 dark:bg-higashi-kashmirblue-900/50 py-2.5 text-gray-900 dark:text-white shadow-sm ring-1 ring-inset ring-gray-300 dark:ring-higashi-kashmirblue-600 placeholder:text-gray-400 dark:placeholder:text-gray-500 focus:ring-2 focus:ring-inset focus:ring-higashi-kashmirblue-500 sm:text-sm sm:leading-6 px-3 transition-colors duration-200"
                  placeholder="••••••••"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
              </div>
            </div>
          </div>

          {message && (
            <div className={`text-sm text-center p-2 rounded-md ${message.type === 'error' ? 'bg-red-100 text-red-700 dark:bg-red-900/50 dark:text-red-200' : 'bg-green-100 text-green-700 dark:bg-green-900/50 dark:text-green-200'}`}>
              {message.text}
            </div>
          )}

          <div>
            <button
              type="submit"
              disabled={loading}
              className="flex w-full justify-center rounded-md bg-higashi-kashmirblue-600 px-3 py-2.5 text-sm font-semibold text-white shadow-sm hover:bg-higashi-kashmirblue-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-higashi-kashmirblue-600 disabled:opacity-50 transition-colors duration-200"
            >
              {loading ? t('Processing...') : isSignUp ? t('Sign up') : t('Sign in')}
            </button>
          </div>
        </form>

        <div className="text-center text-sm">
          <button
            type="button"
            className="font-medium text-higashi-kashmirblue-600 hover:text-higashi-kashmirblue-500 dark:text-higashi-kashmirblue-400 dark:hover:text-higashi-kashmirblue-300 transition-colors"
            onClick={() => {
              setIsSignUp(!isSignUp);
              setMessage(null);
            }}
          >
            {isSignUp
              ? t('Already have an account? Sign in')
              : t("Don't have an account? Sign up")}
          </button>
        </div>
      </div>
    </div>
  );
}
