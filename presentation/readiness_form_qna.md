## Professional Audit 4: Software Testing

**Software Testing
4/4 in Further Vocational Qualification in Information and Communication Technology at ÅYG**

*"...reflect on your experience in project-based work on local-host"*

### Analysis and evaluation of data systems

**What different channels do you use to communicate with your team members?**
As a two person team where both of us happen to live in the same apartment, communication was often direct and in-person and happened either at home or in the classroom where we worked. We also used WhatsApp for messaging and GitHub issues and pull requests and their accompanying discussion functionality. In other projects I've used Jira, emails and other messaging solutions (Discord for example).

**What are the advantages and disadvantages of the different channels you use?**
Being able to communicate in-person helps with bringing up issues that may not arise otherwise and makes coordinating development efficient. Any GitHub discussion has the advantage of leaving a record and being organized by default. Instant messaging is more of a replacement for informal discussions when they're not possible than an organized way of conducting development on its own. I feel it's important to formalize any decisions or plans made in less formal communication into something more persistent by using GitHub or any other project management system.

### Based on a software project you have participated in, answer the following questions:

**Describe the project—its goal, scope, and your role**
The local-host project in the Rust module is about building an HTTP/1.1 server from scratch, without using crates that already implement server features. We are expected to learn about networking, system programming, and HTTP protocol specifics. 

My role was to get some initial bare bones version running, after which we could both go deeper into adding and refining features until we were ready to submit the project. After initial planning and architecture, I focussed on session management and writing tests, among other things.


**What are the functional requirements for different parts of the project?**
The server needs to do be able to serve a static page and support GET, POST, and DELETE methods. It must handle file uploads, cookies and sessions. Crucially, it has to run a modern single-threaded non-blocking I/O epoll equivalent (kqueue for Mac). I must be able to run scripts via CGI and have lots of configuration options for multiple hosts and ports, be robust and come with tests.

**How is the documentation structured to reflect these requirements?**
The assignment on our 01Edu platform outlines the requirements with further details being found in the precise audit questions the project needs to pass. 

Our README.md describes the features of the project, its structure and its usage in detail, including different configurations and testing. There are some clarifying comments in the code and two more markdown files that go into more detail about functionality and testing.

**How do/did you ensure that the desired functionality was actually fulfilled?**
We tested the project in several ways during development. Manual testing was performed when adding features. We wrote unit tests and multiple integration tests as well as conducted stress and memory leak tests. Pull requests to the main branch trigger a testing workflow on GitHub Actions (all but stress and memory leak tests). A new branch cannot be merged without passing those and additionally requires approval from the other group member.

Finally, the project had to pass an audit with specific questions verifying that it actually performs as intended.

**Who performs code reviews? On what basis was this person/these people selected?**
As a two person team, it was the responsibility of the other member to review and approve pull requests. Fellow students reviewed our code during the audits.

Normally a good candidate is a senior or peer developer who is familiar with the relevant codebase.

**Describe the process of designing test plans. Which parts did you actively participate in?**
The project required thorough testing by default, so test plans were present early on.

We included unit tests for new features when feasible. To be able to react to newly introduced bugs, we created integration tests to check if basic functionality (like loading a page) remains in place after updates. By stress testing, we ensured the server didn't become too slow to respond. Regularly testing for memory leaks made sure we caught any nasty bugs early. We kept updating and expanding the tests as the project evolved.

I was responsible for writing the first integration tests and unit tests for configurations and route handling. I wrote instructions on doing memory leak testing, which requires some live monitoring. In later stages I adjusted and rewrote some tests.

### Testing a data system using test automation tools

**Describe how test plans were designed in your project. Which parts did you actively contribute to?**
We took a multi-layered approach to testing, partly due to project requirements. There's unit, integration, stress and memory leak testing. In addition to getting the initial tests going and updating many tests along the way, I set up a GitHub Actions workflow to run tests on pull requests to the main branch.

**What types of test environments have you used?**  
We used local development environments (macOS) and a continuous integration environment in GitHub Actions. Due to our project's kqueue dependency, macOS was the mandatory OS for all environments.

We also set up specialized environments for specific tasks: a memory profiling environment using Rust Nightly/ASan, and a performance testing environment using siege.


**Which testing environments have been easiest for you to set up and work with?**
The local environment, especially the standard Rust toolchain, was easy to work with initially because it provided immediate feedback and didn't require network overhead. Once these tests were in place, setting up the GitHub Actions CI environment was relatively straightforward, although it does require a script to be written correctly and some settings to be made in the remote repository.


**When is manual testing preferred over automated testing? How do you document manual testing?**
Manual testing is a natural way to see if a brand new feature or bugfix under developement is working at all. Exploring how the program works under weird inputs is, at least initially, easiest to do manually. Ceratainly UI and UX has to be tested manually.

The documentation happens in GitHub issues, in the form of reproducible bug reports.


**When is automated testing preferred? How do you document and execute automated tests?**
Automated testing is necessary to catch bugs early on and make sure they don't reappear and to make sure the program stays robust and reliable. Stress or memory tests are completely impractical to perform manually and must be automated if they are to be done. Even for simpler tests, automation makes a much larger amount of testing possible and effortless.

Instructions and descriptions of testing are documented in README.md and in more detail in testing_instructions.md.

Tests are triggered automatically in GitHub and can be run manually with `cargo test`, `go test`, and specific shell scripts provided in the README.

**Once testing is done, who do you report your results to, and how is the information shared?**
GitHub test results are available for all the group in pull request checks and automated CI reports.

Issues in performance found during stress testing (or memory leak testing, but his never occurred) were discussed in person or messaging apps.

**Do you have any questions or comments before the Professional Audit 4 session?**
-
