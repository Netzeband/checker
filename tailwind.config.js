/** @type {import('tailwindcss').Config} */
module.exports = {
    content: ["*.html", "./app/src/**/*.rs",],
    daisyui: {
        themes: ["light", "dark", "cupcake"],
    },
    theme: {
        extend: {},
    },
    plugins: [
        require("@tailwindcss/typography"),
        require('daisyui'),
    ],
}
