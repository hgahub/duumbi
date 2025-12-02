-- Create a table for public profiles (extends Supabase Auth)
create table public.profiles (
  id uuid references auth.users not null primary key,
  updated_at timestamp with time zone,
  username text unique,
  full_name text,
  avatar_url text,
  website text,

  constraint username_length check (char_length(username) >= 3)
);

alter table public.profiles enable row level security;

create policy "Public profiles are viewable by everyone."
  on public.profiles for select
  using ( true );

create policy "Users can insert their own profile."
  on public.profiles for insert
  with check ( auth.uid() = id );

create policy "Users can update own profile."
  on public.profiles for update
  using ( auth.uid() = id );

-- Create a table for properties
create table public.properties (
  id uuid default gen_random_uuid() primary key,
  created_at timestamp with time zone default timezone('utc'::text, now()) not null,
  updated_at timestamp with time zone default timezone('utc'::text, now()) not null,
  owner_id uuid references public.profiles(id) not null,

  title text not null,
  description text,
  price numeric not null,
  currency text default 'HUF',

  location_city text not null,
  location_address text,
  location_lat float,
  location_lng float,

  features jsonb default '{}'::jsonb, -- Flexible storage for features (e.g., {"bedrooms": 2, "parking": true})
  images text[] default '{}', -- Array of image URLs

  status text default 'active' check (status in ('active', 'sold', 'archived'))
);

alter table public.properties enable row level security;

create policy "Properties are viewable by everyone."
  on public.properties for select
  using ( true );

create policy "Users can insert their own properties."
  on public.properties for insert
  with check ( auth.uid() = owner_id );

create policy "Users can update their own properties."
  on public.properties for update
  using ( auth.uid() = owner_id );

-- Create a table for saved searches
create table public.saved_searches (
  id uuid default gen_random_uuid() primary key,
  created_at timestamp with time zone default timezone('utc'::text, now()) not null,
  user_id uuid references public.profiles(id) not null,

  name text not null,
  criteria jsonb not null, -- Store search filters here
  notify_email boolean default true
);

alter table public.saved_searches enable row level security;

create policy "Users can view their own saved searches."
  on public.saved_searches for select
  using ( auth.uid() = user_id );

create policy "Users can insert their own saved searches."
  on public.saved_searches for insert
  with check ( auth.uid() = user_id );

create policy "Users can update their own saved searches."
  on public.saved_searches for update
  using ( auth.uid() = user_id );

create policy "Users can delete their own saved searches."
  on public.saved_searches for delete
  using ( auth.uid() = user_id );

-- Set up Realtime for properties
alter publication supabase_realtime add table public.properties;
