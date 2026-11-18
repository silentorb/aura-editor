import { createTheme } from "@mantine/core";

/** Dense desktop-productivity defaults (IDE-like), not marketing spacing. */
export const theme = createTheme({
  fontFamily:
    'Segoe UI, "Helvetica Neue", Helvetica, Arial, sans-serif',
  defaultRadius: "sm",
  spacing: {
    xs: "4px",
    sm: "8px",
    md: "12px",
    lg: "16px",
    xl: "24px",
  },
  headings: {
    fontWeight: "600",
  },
  components: {
    Button: {
      defaultProps: {
        size: "compact-sm",
      },
    },
    Tabs: {
      defaultProps: {
        variant: "default",
      },
    },
  },
});
