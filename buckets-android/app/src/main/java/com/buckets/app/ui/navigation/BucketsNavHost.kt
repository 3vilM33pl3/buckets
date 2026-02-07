// SPDX-License-Identifier: MIT
package com.buckets.app.ui.navigation

import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.material3.MaterialTheme
import androidx.navigation.NavDestination.Companion.hierarchy
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.NavHostController
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.buckets.app.ui.buckets.BucketDetailScreen
import com.buckets.app.ui.buckets.BucketsListScreen
import com.buckets.app.ui.commits.CommitDetailScreen
import com.buckets.app.ui.commits.CommitHistoryScreen
import com.buckets.app.ui.expectations.ExpectationsScreen
import com.buckets.app.ui.graph.GraphScreen
import com.buckets.app.ui.pebbles.PebblesScreen
import com.buckets.app.ui.settings.SettingsScreen

private data class BottomNavItem(
    val label: String,
    val icon: ImageVector,
    val route: String,
)

private val bottomNavItems = listOf(
    BottomNavItem("Buckets", Icons.AutoMirrored.Filled.List, Screen.BucketsList.route),
    BottomNavItem("Expectations", Icons.Filled.CheckCircle, Screen.ExpectationsList.route),
    BottomNavItem("Pebbles", Icons.Filled.Star, Screen.PebblesList.route),
    BottomNavItem("Graph", Icons.Filled.Share, Screen.Graph.route),
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BucketsNavHost(
    navController: NavHostController = rememberNavController(),
) {
    val navBackStackEntry by navController.currentBackStackEntryAsState()
    val currentDestination = navBackStackEntry?.destination

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Buckets") },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primary,
                    titleContentColor = MaterialTheme.colorScheme.onPrimary,
                    actionIconContentColor = MaterialTheme.colorScheme.onPrimary,
                ),
                actions = {
                    IconButton(onClick = {
                        navController.navigate(Screen.Settings.route) {
                            launchSingleTop = true
                        }
                    }) {
                        Icon(Icons.Filled.Settings, contentDescription = "Settings")
                    }
                },
            )
        },
        bottomBar = {
            NavigationBar {
                bottomNavItems.forEach { item ->
                    NavigationBarItem(
                        icon = { Icon(item.icon, contentDescription = item.label) },
                        label = { Text(item.label) },
                        selected = currentDestination?.hierarchy?.any { it.route == item.route } == true,
                        onClick = {
                            navController.navigate(item.route) {
                                popUpTo(navController.graph.findStartDestination().id) {
                                    saveState = true
                                }
                                launchSingleTop = true
                                restoreState = true
                            }
                        },
                    )
                }
            }
        },
    ) { innerPadding ->
        NavHost(
            navController = navController,
            startDestination = Screen.BucketsList.route,
            modifier = Modifier.padding(innerPadding),
        ) {
            composable(Screen.BucketsList.route) {
                BucketsListScreen(
                    onBucketClick = { id ->
                        navController.navigate(Screen.BucketDetail.createRoute(id))
                    },
                )
            }

            composable(
                route = Screen.BucketDetail.ROUTE,
                arguments = listOf(navArgument("id") { type = NavType.StringType }),
            ) { backStackEntry ->
                val id = backStackEntry.arguments?.getString("id").orEmpty()
                BucketDetailScreen(
                    bucketId = id,
                    onCommitClick = { commitId ->
                        navController.navigate(Screen.CommitDetail.createRoute(commitId))
                    },
                    onViewAllCommits = {
                        navController.navigate(Screen.CommitHistory.createRoute(id))
                    },
                    onNavigateBack = { navController.popBackStack() },
                )
            }

            composable(
                route = Screen.CommitHistory.ROUTE,
                arguments = listOf(navArgument("bucketId") { type = NavType.StringType }),
            ) { backStackEntry ->
                val bucketId = backStackEntry.arguments?.getString("bucketId").orEmpty()
                CommitHistoryScreen(
                    bucketId = bucketId,
                    onCommitClick = { commitId ->
                        navController.navigate(Screen.CommitDetail.createRoute(commitId))
                    },
                    onNavigateBack = { navController.popBackStack() },
                )
            }

            composable(
                route = Screen.CommitDetail.ROUTE,
                arguments = listOf(navArgument("commitId") { type = NavType.StringType }),
            ) { backStackEntry ->
                val commitId = backStackEntry.arguments?.getString("commitId").orEmpty()
                CommitDetailScreen(
                    commitId = commitId,
                    onNavigateBack = { navController.popBackStack() },
                )
            }

            composable(Screen.ExpectationsList.route) {
                ExpectationsScreen()
            }

            composable(Screen.PebblesList.route) {
                PebblesScreen()
            }

            composable(Screen.Graph.route) {
                GraphScreen()
            }

            composable(Screen.Settings.route) {
                SettingsScreen(
                    onNavigateBack = { navController.popBackStack() },
                )
            }
        }
    }
}
