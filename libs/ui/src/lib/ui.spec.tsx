import { render } from '@testing-library/react';

import DuumbiUiComponents from './ui';

describe('DuumbiUiComponents', () => {
  it('should render successfully', () => {
    const { baseElement } = render(<DuumbiUiComponents />);
    expect(baseElement).toBeTruthy();
  });
});
