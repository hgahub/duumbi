declare const _default: {
    displayName: string;
    preset: string;
    transform: {
        '^(?!.*\\.(js|jsx|ts|tsx|css|json)$)': string;
        '^.+\\.[tj]sx?$': (string | {
            presets: string[];
        })[];
    };
    moduleFileExtensions: string[];
    coverageDirectory: string;
};
export default _default;
//# sourceMappingURL=jest.config.d.ts.map